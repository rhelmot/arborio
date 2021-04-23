use super::map_struct;
use fltk::{prelude::*,*};

use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::{min, max};

pub struct EditorWidget {
    state: Rc<RefCell<EditorState>>,
    pub widget: widget::Widget,
}

struct EditorState {
    map: Option<map_struct::CelesteMap>,
    current_room: usize,
    map_corner_x: i32,
    map_corner_y: i32,
    map_scale: u32, // screen pixels per game tile
}

impl EditorWidget {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> EditorWidget {
        let state = EditorState {
            map: None,
            current_room: 0,
            map_corner_x: 0,
            map_corner_y: 0,
            map_scale: 8,
        };

        let mut result = EditorWidget {
            state: Rc::new(RefCell::new(state)),
            widget: widget::Widget::new(x, y, w, h, ""),
        };

        result.draw();
        result.handle();

        return result;
    }

    pub fn set_map(&mut self, map: map_struct::CelesteMap) {
        self.state.borrow_mut().map = Some(map);
        self.widget.redraw();
    }

    fn draw(&mut self) {
        let state = self.state.clone();
        self.widget.draw(move |b| {
            let state = state.borrow();
            let screen = map_struct::CelesteMapRect::from_widget(b);
            draw::push_clip(b.x(), b.y(), b.w(), b.h());
            draw::draw_rect_fill(b.x(), b.y(), b.w(), b.h(), enums::Color::White);

            match &state.map {
                Some(map) => {
                    for filler in map.filler.iter() {
                        let filler_screen = state.rect_level_to_screen(filler);
                        if filler_screen.intersects(&screen) {
                            draw::draw_rect_fill(
                                filler_screen.x,
                                filler_screen.y,
                                filler_screen.width as i32,
                                filler_screen.height as i32,
                                enums::Color::from_u32(0x804000))
                        }
                    }
                    for room in map.levels.iter() {
                        let rect_screen = state.rect_level_to_screen(&room.bounds);
                        if rect_screen.intersects(&screen) {
                            draw::draw_rect_fill(
                                rect_screen.x,
                                rect_screen.y,
                                rect_screen.width as i32,
                                rect_screen.height as i32,
                                enums::Color::from_u32(0x3020c0))
                        }
                    }
                }
                _ => (),
            }
            draw::pop_clip();
        });
    }

    fn handle(&mut self) {
        let state = self.state.clone();
        self.widget.handle(move |b, ev| {
            let mut state = state.borrow_mut();
            match ev {
                enums::Event::Enter => {
                    true
                },
                enums::Event::MouseWheel => {
                    let ydir = match app::event_dy() {
                        app::MouseWheel::Down => -1,
                        app::MouseWheel::Up => 1,
                        _ => 0,
                    };
                    let xdir: i32 = match app::event_dx() {
                        app::MouseWheel::Right => -1,
                        app::MouseWheel::Left => 1,
                        _ => 0,
                    };
                    if app::event_key_down(enums::Key::ControlL) || app::event_key_down(enums::Key::ControlR) {
                        let (old_x, old_y) = state.point_screen_to_level(app::event_x(), app::event_y());
                        state.map_scale = max(min(30, state.map_scale as i32 + ydir), 1) as u32;
                        let (new_x, new_y) = state.point_screen_to_level(app::event_x(), app::event_y());
                        state.map_corner_x += old_x - new_x;
                        state.map_corner_y += old_y - new_y;
                    } else {
                        let amt = state.size_screen_to_level(30);
                        state.map_corner_x += amt as i32 * xdir;
                        state.map_corner_y += amt as i32 * ydir;
                    }
                    b.redraw();
                    true
                },
                _ => false
            }
        });
    }
}

impl EditorState {
    fn rect_level_to_screen(&self, rect: &map_struct::CelesteMapRect) -> map_struct::CelesteMapRect {
        let (x, y) = self.point_level_to_screen(rect.x, rect.y);
        map_struct::CelesteMapRect {
            x, y,
            width: self.size_level_to_screen(rect.width),
            height: self.size_level_to_screen(rect.height),
        }
    }

    fn rect_screen_to_level(&self, rect: map_struct::CelesteMapRect) -> map_struct::CelesteMapRect {
        let (x, y) = self.point_screen_to_level(rect.x, rect.y);
        map_struct::CelesteMapRect {
            x, y,
            width: self.size_screen_to_level(rect.width),
            height: self.size_screen_to_level(rect.height),
        }
    }

    fn size_level_to_screen(&self, size: u32) -> u32 {
        size * self.map_scale / 8
    }

    fn size_screen_to_level(&self, size: u32) -> u32 {
        size * 8 / self.map_scale
    }

    fn point_screen_to_level(&self, x: i32, y: i32) -> (i32, i32) {
        (x * 8 / self.map_scale as i32 + self.map_corner_x,
         y * 8 / self.map_scale as i32 + self.map_corner_y)
    }

    fn point_level_to_screen(&self, x: i32, y: i32) -> (i32, i32) {
        ((x - self.map_corner_x) * self.map_scale as i32 / 8,
         (y - self.map_corner_y) * self.map_scale as i32 / 8)
    }
}

impl map_struct::CelesteMapRect {
    pub fn intersects(&self, other: &map_struct::CelesteMapRect) -> bool {
        (   (self.x >= other.x && self.x < other.x + other.width as i32) ||
            (other.x >= self.x && other.x < self.x + self.width as i32)) && (
            (self.y >= other.y && self.y < other.y + other.height as i32) ||
            (other.y >= self.y && other.y < self.y + self.height as i32))
    }

    pub fn from_widget(wid: &widget::Widget) -> map_struct::CelesteMapRect {
        map_struct::CelesteMapRect {
            x: wid.x(),
            y: wid.y(),
            width: wid.w() as u32,
            height: wid.h() as u32,
        }
    }
}