use crate::advanced_tweaker::attr_editor;
use arborio_maploader::map_struct::Attribute;
use arborio_modloader::config::{AttributeInfo, AttributeType};
use arborio_utils::vizia::fonts::icons_names::DOWN;
use arborio_utils::vizia::prelude::*;

pub fn basic_attrs_editor<LN, F, LK, LA, LC, FS>(
    cx: &mut Context,
    lens_count: LN,
    index_func: F,
    setter: FS,
) where
    LN: Clone + Send + Sync + Lens<Target = usize>,
    F: 'static + Clone + Send + Sync + Fn(usize) -> (LK, LA, LC),
    LK: Send + Sync + Lens<Target = String>,
    LA: Send + Sync + Lens<Target = Attribute>,
    LC: Send + Sync + Lens<Target = AttributeInfo>,
    FS: 'static + Clone + Send + Sync + Fn(&mut EventContext, String, Attribute),
{
    Binding::new(cx, lens_count, move |cx, info_len| {
        let info_len = info_len.get_fallible(cx).unwrap_or_default();

        for idx in 0..info_len {
            let (lens_attr_key, lens_attr_val, lens_attr_info) = index_func(idx);
            let lens_attr_type = lens_attr_info.clone().then(AttributeInfo::ty);
            let lens_attr_name = lens_attr_info.clone().then(AttributeInfo::display_name);
            let lens_attr_opts = lens_attr_info.clone().then(AttributeInfo::options);

            if lens_attr_info.then(AttributeInfo::ignore).get(cx) {
                continue;
            }

            let setter = setter.clone();
            HStack::new(cx, move |cx| {
                {
                    let lens_attr_key = lens_attr_key.clone();
                    let lens_attr_name = lens_attr_name.clone();
                    Binding::new(cx, lens_attr_key, move |cx, lens_attr_key| {
                        let attr_key = lens_attr_key.get(cx);
                        let lens_attr_name = lens_attr_name.clone();
                        Binding::new(cx, lens_attr_name, move |cx, lens_attr_name| {
                            let attr_name = lens_attr_name.get(cx);
                            let name = attr_name.as_ref().unwrap_or(&attr_key);
                            Label::new(cx, name);
                        });
                    });
                }
                let lens_attr_val = lens_attr_val.clone();
                Binding::new(cx, lens_attr_opts, move |cx, lens_attr_opts| {
                    let lens_attr_val = lens_attr_val.clone();
                    let lens_attr_key = lens_attr_key.clone();
                    let setter = setter.clone();
                    let opts_len =
                        lens_attr_opts.view(cx.data().unwrap(), |opts| opts.map_or(0, |x| x.len()));
                    if opts_len != 0 {
                        // ugh... the placement of this binding is nontrivial
                        Binding::new(cx, lens_attr_val, move |cx, lens_attr_val| {
                            let attr_val = lens_attr_val.get_fallible(cx);
                            let (found_idx, found_lbl) =
                                lens_attr_opts.view(cx.data().unwrap(), |opts| {
                                    for (idx, opt) in opts.unwrap().iter().enumerate() {
                                        if let Some(attr_val) = &attr_val {
                                            if opt.value.to_binel().eq_insensitive(attr_val) {
                                                return (Some(idx), Some(opt.name.clone()));
                                            }
                                        }
                                    }
                                    (None, None)
                                });
                            let lens_attr_opts = lens_attr_opts.clone();
                            let lens_attr_key = lens_attr_key.clone();
                            let setter = setter.clone();
                            Dropdown::new(
                                cx,
                                move |cx| {
                                    let found_lbl = found_lbl.clone();
                                    HStack::new(cx, move |cx| {
                                        Label::new(
                                            cx,
                                            found_lbl.as_ref().map_or("weh", |a| a.as_str()),
                                        );
                                        Label::new(cx, DOWN).font_family(vec![FamilyOwned::Name(
                                            "Entypo".to_owned(),
                                        )]);
                                    })
                                    .width(Stretch(1.0))
                                },
                                move |cx| {
                                    for idx in 0..opts_len {
                                        let opt = lens_attr_opts.clone().index(idx).get(cx);
                                        let lens_attr_key = lens_attr_key.clone();
                                        let setter = setter.clone();
                                        Label::new(cx, &opt.name)
                                            .class("dropdown_element")
                                            .toggle_class("checked", Some(idx) == found_idx)
                                            .on_press(move |cx| {
                                                let key = lens_attr_key.get(cx);
                                                setter(cx.as_mut(), key, opt.value.to_binel());
                                                cx.emit(PopupEvent::Close);
                                            });
                                    }
                                },
                            );
                        });
                    } else {
                        Binding::new(cx, lens_attr_type.clone(), move |cx, attr_type| {
                            let lens_attr_val = lens_attr_val.clone();
                            let lens_attr_key = lens_attr_key.clone();
                            let setter = setter.clone();
                            let attr_type = attr_type.get(cx);
                            match attr_type {
                                AttributeType::String => {
                                    attr_editor(
                                        cx,
                                        lens_attr_val.then(Attribute::text),
                                        lens_attr_key,
                                        move |cx, key, val| {
                                            setter(cx, key, Attribute::Text(val));
                                        },
                                        true,
                                    );
                                }
                                AttributeType::Int => {
                                    attr_editor(
                                        cx,
                                        lens_attr_val.then(Attribute::int),
                                        lens_attr_key,
                                        move |cx, key, val| {
                                            setter(cx, key, Attribute::Int(val));
                                        },
                                        true,
                                    );
                                }
                                AttributeType::Float => {
                                    attr_editor(
                                        cx,
                                        lens_attr_val.then(Attribute::float),
                                        lens_attr_key,
                                        move |cx, key, val| {
                                            setter(cx, key, Attribute::Float(val));
                                        },
                                        true,
                                    );
                                }
                                AttributeType::Bool => {
                                    Checkbox::new(cx, lens_attr_val.clone().then(Attribute::bool))
                                        .on_toggle(move |cx| {
                                            let key = lens_attr_key.get(cx);
                                            let val = !lens_attr_val
                                                .clone()
                                                .then(Attribute::bool)
                                                .get_fallible(cx)
                                                .unwrap_or(false);
                                            setter(cx, key, Attribute::Bool(val));
                                        });
                                }
                            }
                        });
                    }
                });
            });
        }

        // let attrs_len_lens = lens_attributes.then(HashMapLenLens::new());
        // Binding::new(cx, attrs_len_lens, move |cx, _| {
        //     let keys_attributes = lens_attributes.view(cx.data().unwrap(), |val| {
        //         val.map_or(HashSet::default(), |x| {
        //             x.keys().cloned().collect::<HashSet<String>>()
        //         })
        //     });
        //     let keys_config = lens_config.view(cx.data().unwrap(), |val| {
        //         val.map_or(HashSet::default(), |x| {
        //             x.keys().cloned().collect::<HashSet<String>>()
        //         })
        //     });
        //     let difference = keys_attributes.difference(&keys_config).count();
        //     if difference != 0 {
        //         Label::new(
        //             cx,
        //             &format!(
        //                 "{} attribute{} are not configured.",
        //                 difference,
        //                 if difference == 1 { "s" } else { "" }
        //             ),
        //         );
        //         Label::new(cx, "Do you need to use advanced mode?");
        //     }
        // })
    })
}
