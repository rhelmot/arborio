use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use vizia::*;

use crate::app_state::AppState;
use crate::assets::Interned;
use crate::celeste_mod::entity_config::{EntityConfig, TriggerConfig};
use crate::logging::*;
use crate::map_struct::{CelesteMapEntity, Node};
use crate::units::*;
use crate::widgets::editor;

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
        <LI as Lens>::Source: Model,
        <LL as Lens>::Source: Model,
    {
        let result = Self {
            lens: selected.clone(),
            marker: PhantomData {},
        }
        .build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                List::new(cx, items, move |cx, _, item| {
                    let item2 = item.clone();
                    let item3 = item.clone();
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "").bind(item2, |handle, item| {
                            let app = handle.cx.data().unwrap();
                            let text = item.get(handle.cx).display_name(app);
                            handle.text(&text);
                        });
                    })
                    .class("palette_item")
                    .bind(selected.clone(), move |handle, selected| {
                        let mine = item3.get(handle.cx);
                        let selected = selected.get(handle.cx);
                        handle.checked(selected.same(&mine));
                    })
                    .on_press(move |cx| {
                        (callback)(cx, item.get(cx));
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
        let data = self
            .lens
            .view(cx.data::<<L as Lens>::Source>().unwrap(), |x| *x.unwrap());

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        canvas.scissor(0.0, 0.0, bounds.w, 100.0);

        let mut path = vg::Path::new();
        path.rect(0.0, 0.0, bounds.w, 100.0);
        canvas.fill_path(
            &mut path,
            vg::Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                100.0,
                Color::black().into(),
                Color::blue().into(),
            ),
        );

        data.draw(cx.data::<AppState>().unwrap(), canvas).emit(cx);
        canvas.restore();
    }
}

pub trait PaletteItem: Copy + Clone + Data + Debug + Send {
    fn search_text(&self) -> String;
    fn display_name(&self, app: &AppState) -> String;
    const CAN_DRAW: bool = true;
    fn draw(&self, app: &AppState, canvas: &mut Canvas) -> LogResult<()>;
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
            texture: None,
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

    fn display_name(&self, _app: &AppState) -> String {
        self.name.to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) -> LogResult<()> {
        let mut log = LogBuf::new();
        if let Some(texture) = self.texture {
            if !app.map_tab_check() {
                println!("SOMETHING IS WRONG (list)");
            } else {
                canvas.scale(3.0, 3.0);
                app.current_palette_unwrap()
                    .gameplay_atlas
                    .draw_sprite(
                        canvas,
                        texture,
                        Point2D::zero(),
                        None,
                        Some(Vector2D::zero()),
                        None,
                        None,
                        0.0,
                    )
                    .offload(LogLevel::Error, &mut log);
            }
        }

        log.done(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntitySelectable {
    pub entity: Interned,
    pub template: usize,
}

impl Default for EntitySelectable {
    fn default() -> Self {
        Self {
            entity: "does not exist".into(),
            template: 0,
        }
    }
}

impl Default for TriggerSelectable {
    fn default() -> Self {
        Self {
            trigger: "does not exist".into(),
            template: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TriggerSelectable {
    pub trigger: Interned,
    pub template: usize,
}

impl Data for EntitySelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for TriggerSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PaletteItem for EntitySelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, app: &AppState) -> String {
        (*self.config(app).templates[self.template].name).to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) -> LogResult<()> {
        canvas.scale(2.0, 2.0);

        let tmp_entity = self.instantiate(
            app,
            16,
            16,
            self.config(app).minimum_size_x as i32,
            self.config(app).minimum_size_y as i32,
            vec![(48, 16).into()],
        );
        editor::draw_entity(
            app,
            canvas,
            &tmp_entity,
            &TileGrid::empty(),
            false,
            false,
            &TileGrid::empty(),
        )
    }
}

impl PaletteItem for TriggerSelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, app: &AppState) -> String {
        (*self.config(app).templates[self.template].name).to_owned()
    }

    const CAN_DRAW: bool = false;
    fn draw(&self, _app: &AppState, _canvas: &mut Canvas) -> LogResult<()> {
        panic!("You cannot draw a trigger. don't call me!")
    }
}

impl EntitySelectable {
    pub fn config<'a>(&self, app: &'a AppState) -> &'a Arc<EntityConfig> {
        app.current_palette_unwrap()
            .get_entity_config(*self.entity, false)
    }

    pub fn instantiate(
        &self,
        app: &AppState,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        nodes: Vec<Node>,
    ) -> CelesteMapEntity {
        let config = self.config(app);

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
        let width = width.max(config.minimum_size_x);
        let height = height.max(config.minimum_size_y);
        let width = if !config.resizable_x {
            config.minimum_size_x
        } else {
            width
        };
        let height = if !config.resizable_y {
            config.minimum_size_y
        } else {
            height
        };

        let mut entity = CelesteMapEntity {
            id: 0,
            name: config.entity_name.to_string(),
            attributes: config.templates[self.template]
                .attributes
                .iter()
                .map(|attr| (attr.0.to_string(), attr.1.to_binel()))
                .collect(),
            x,
            y,
            width,
            height,
            nodes,
        };
        for (attr, info) in config.attribute_info.iter() {
            if !entity.attributes.contains_key(attr) {
                entity
                    .attributes
                    .insert(attr.to_string(), info.default.to_binel());
            }
        }

        entity
    }
}

impl TriggerSelectable {
    pub fn config<'a>(&self, app: &'a AppState) -> &'a Arc<TriggerConfig> {
        let palette = app.current_palette_unwrap();
        palette
            .trigger_config
            .get(&self.trigger)
            .unwrap_or_else(|| palette.trigger_config.get("default").unwrap())
    }

    pub fn instantiate(
        &self,
        app: &AppState,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        nodes: Vec<Node>,
    ) -> CelesteMapEntity {
        let config = self.config(app);

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
            name: config.trigger_name.to_string(),
            attributes: config.templates[self.template]
                .attributes
                .iter()
                .map(|attr| (attr.0.to_string(), attr.1.to_binel()))
                .collect(),
            x,
            y,
            width,
            height,
            nodes,
        };
        for (attr, info) in config.attribute_info.iter() {
            if !entity.attributes.contains_key(attr) {
                entity
                    .attributes
                    .insert(attr.to_string(), info.default.to_binel());
            }
        }

        entity
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DecalSelectable(pub Interned);

impl Data for DecalSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PaletteItem for DecalSelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, _app: &AppState) -> String {
        self.0.to_string()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) -> LogResult<()> {
        let mut log = LogBuf::new();
        app.current_palette_unwrap()
            .gameplay_atlas
            .draw_sprite(
                canvas,
                &format!("decals/{}", self.0),
                Point2D::new(0.0, 0.0),
                None,
                Some(Vector2D::zero()),
                None,
                None,
                0.0,
            )
            .offload(LogLevel::Error, &mut log);
        log.done(())
    }
}

impl DecalSelectable {
    pub fn new(path: Interned) -> Self {
        Self(path)
    }
}

impl Default for DecalSelectable {
    fn default() -> Self {
        Self("does not exist".into())
    }
}
