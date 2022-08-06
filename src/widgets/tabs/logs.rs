use crate::logging::ArborioRecord;
use crate::AppState;
use log::Level;
use std::collections::HashMap;
use vizia::prelude::*;

pub fn build_logs(cx: &mut Context) {
    ScrollView::new(cx, 0.0, 1.0, false, true, |cx| {
        Binding::new(cx, AppState::logs.map(|logs| logs.len()), |cx, _| {
            let mut annotated: Vec<(usize, ArborioRecord)> = vec![];

            AppState::logs.view(cx.data().unwrap(), |logs| {
                let mut count = HashMap::new();
                if let Some(logs) = logs {
                    for message in logs.iter() {
                        *count.entry(message).or_insert(0) += 1;
                    }
                    for message in logs.iter() {
                        if let Some(ct) = count.remove(message) {
                            annotated.push((ct, message.clone()));
                        }
                    }
                }
            });

            for (count, message) in annotated {
                let count_text = if count > 1 {
                    count.to_string()
                } else {
                    "".to_owned()
                };
                HStack::new(cx, move |cx| {
                    Label::new(cx, &count_text).class("log_icon");
                    Label::new(cx, &message.message).class("log_text");
                })
                .class(match message.level {
                    Level::Error => "error",
                    Level::Warn => "warning",
                    Level::Info => "info",
                    Level::Debug => "debug",
                    Level::Trace => "trace",
                })
                .class("log_entry");
            }
        });
    });
}
