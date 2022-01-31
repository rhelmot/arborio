use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
use vizia::*;

use crate::atlas_img::SpriteReference;
use crate::assets;
use crate::config::entity_config::{EntityConfig, EntityTemplate, TriggerConfig};
use crate::map_struct::CelesteMapEntity;
use crate::units::*;
use crate::widgets::editor_widget;

pub struct PaletteWidget<T, L> {
    lens: L,
    marker: PhantomData<T>,
}

impl<T: PaletteItem, LI> PaletteWidget<T, LI>
where
    LI: Lens<Target = T>,
{
    pub fn new<F, LL>(cx: &mut Context, items: LL, selected: LI, callback: F) -> Handle<Self>
    where
        F: 'static + Fn(&mut Context, T) + Copy,
        LL: Lens<Target = Vec<T>>,
    {
        let result = Self { lens: selected, marker: PhantomData {} }
            .build2(cx, move |cx| {
                List::new(cx, items, move |cx, item| {
                    Binding::new(cx, selected, move |cx, selected_field| {
                        let selected = *selected_field.get(cx);
                        let checked = item.get(cx).same(&selected);
                        HStack::new(cx, move |cx| {
                            Label::new(cx, &item.get(cx).display_name());
                        })
                            .class("palette_item")
                            .checked(checked)
                            .on_press(move |cx| {
                                (callback)(cx, *item.get(cx));
                            });
                    });
                });
            });


        if T::CAN_DRAW {
            result.child_top(Units::Pixels(100.0))
        } else {
            result
        }
    }
}

impl<T: PaletteItem, L: Lens<Target = T>> View for PaletteWidget<T, L> {
    fn element(&self) -> Option<String> {
        Some("palette".to_owned())
    }

    fn draw(&self, cx: &mut Context, canvas: &mut Canvas) {
        if !T::CAN_DRAW {
            return;
        }

        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);
        let data = self.lens.view(cx.data::<<L as Lens>::Source>().unwrap()).unwrap();

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        canvas.scissor(0.0, 0.0, bounds.w, 100.0);

        let mut path = femtovg::Path::new();
        path.rect(0.0, 0.0, bounds.w, 100.0);
        canvas.fill_path(&mut path, femtovg::Paint::linear_gradient(
            0.0, 0.0, 0.0, 100.0,
            Color::black().into(), Color::blue().into())
        );

        data.draw(canvas);
        canvas.restore();
    }
}

pub trait PaletteItem: Copy + Clone + Data + Debug + Send {
    fn search_text(&self) -> String;
    fn display_name(&self) -> String;
    const CAN_DRAW: bool = true;
    fn draw(&self, canvas: &mut Canvas);
}

#[derive(Copy, Clone, Debug)]
pub struct TileSelectable {
    pub id: char,
    pub name: &'static str,
    pub texture: Option<&'static str>,
}

impl Default for TileSelectable {
    fn default() -> Self {
        TileSelectable {
            id: '0',
            name: "Empty",
            texture: None
        }
    }
}

impl PartialEq for TileSelectable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

unsafe impl Send for TileSelectable {}

impl Data for TileSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PaletteItem for TileSelectable {
    fn search_text(&self) -> String {
        self.name.to_owned()
    }

    fn display_name(&self) -> String {
        self.name.to_owned()
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.scale(3.0, 3.0);
        if let Some(texture) = self.texture {
            let texture = assets::GAMEPLAY_ATLAS.lookup(texture).unwrap();
            assets::GAMEPLAY_ATLAS.draw_sprite(canvas, texture, Point2D::zero(), None, Some(Vector2D::zero()), None, None, 0.0);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EntitySelectable<'a> {
    pub config: &'a EntityConfig,
    pub template: &'a EntityTemplate,
}

#[derive(Copy, Clone, Debug)]
pub struct TriggerSelectable<'a> {
    pub config: &'a TriggerConfig,
    pub template: &'a EntityTemplate,
}

impl Data for EntitySelectable<'static> {
    fn same(&self, other: &Self) -> bool {
        // ummmmmm is this a good idea
        self.template as *const EntityTemplate == other.template as *const EntityTemplate
    }
}

impl Data for TriggerSelectable<'static> {
    fn same(&self, other: &Self) -> bool {
        // ummmmmm is this a good idea
        self.template as *const EntityTemplate == other.template as *const EntityTemplate
    }
}

impl PaletteItem for EntitySelectable<'static> {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self) -> String {
        self.template.name.clone()
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.scale(2.0, 2.0);

        let tmp_entity = self.instantiate(
            16, 16,
            self.config.minimum_size_x as i32, self.config.minimum_size_y as i32,
            vec![(48, 16)]
        );
        editor_widget::draw_entity(canvas, &tmp_entity, &TileGrid::empty(), false, false);
    }
}

impl PaletteItem for TriggerSelectable<'static> {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self) -> String {
        self.template.name.clone()
    }

    const CAN_DRAW: bool = false;
    fn draw(&self, canvas: &mut Canvas) {
        panic!("You cannot draw a trigger. don't call me!")
    }
}

impl EntitySelectable<'_> {
    pub fn instantiate(&self, x: i32, y: i32, width: i32, height: i32, nodes: Vec<(i32, i32)>) -> CelesteMapEntity {
        let (x, width) = if width < 0 {
            (x + width, -width as u32)
        } else {
            (x, width as u32)
        };
        let (y, height) = if height < 0 {
            (y + height, -height as u32)
        } else {
            (y, height as u32)
        };
        let width = width.max(self.config.minimum_size_x);
        let height = height.max(self.config.minimum_size_y);
        let width = if !self.config.resizable_x { self.config.minimum_size_x } else { width };
        let height = if !self.config.resizable_y { self.config.minimum_size_y } else { height };

        let mut entity = CelesteMapEntity {
            id: 0,
            name: self.config.entity_name.clone(),
            attributes: self.template.attributes.iter().map(|attr| (attr.0.clone(), attr.1.to_binel())).collect(),
            x, y, width, height, nodes,
        };
        for (attr, info) in &self.config.attribute_info {
            if !entity.attributes.contains_key(attr) {
                entity.attributes.insert(attr.clone(), info.default.to_binel());
            }
        }

        entity
    }
}

impl TriggerSelectable<'_> {
    pub fn instantiate(&self, x: i32, y: i32, width: i32, height: i32, nodes: Vec<(i32, i32)>) -> CelesteMapEntity {
        let (x, width) = if width < 0 {
            (x + width, -width as u32)
        } else {
            (x, width as u32)
        };
        let (y, height) = if height < 0 {
            (y + height, -height as u32)
        } else {
            (y, height as u32)
        };
        let width = width.max(8);
        let height = height.max(8);

        let mut entity = CelesteMapEntity {
            id: 0,
            name: self.config.trigger_name.clone(),
            attributes: self.template.attributes.iter().map(|attr| (attr.0.clone(), attr.1.to_binel())).collect(),
            x, y, width, height, nodes,
        };
        for (attr, info) in &self.config.attribute_info {
            if !entity.attributes.contains_key(attr) {
                entity.attributes.insert(attr.clone(), info.default.to_binel());
            }
        }

        entity
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DecalSelectable(pub &'static str);

impl Data for DecalSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PaletteItem for DecalSelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self) -> String {
        self.0.to_owned()
    }

    fn draw(&self, canvas: &mut Canvas) {
        assets::GAMEPLAY_ATLAS.draw_sprite(
            canvas,
            assets::GAMEPLAY_ATLAS.lookup(&("decals/".to_owned() + self.0)).unwrap(),
            Point2D::new(0.0, 0.0),
            None, Some(Vector2D::zero()), None, None, 0.0,
        )
    }
}

impl DecalSelectable {
    pub fn new(path: &'static str) -> Self {
        Self(path)
    }
}
