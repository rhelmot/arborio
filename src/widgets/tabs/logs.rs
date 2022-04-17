use vizia::*;

pub fn build_logs(cx: &mut Context) {
    ScrollView::new(cx, 0.0, 100.0, false, true, |_cx| {
        // TODO
        // Binding::new(cx, AppState::logs.map(|logs| logs.len()), |cx, _| {
        //     let mut annotated = vec![];
        //
        //     AppState::logs.view(cx.data().unwrap(), |logs| {
        //         let mut count = HashMap::new();
        //         if let Some(logs) = logs {
        //             for message in logs.iter() {
        //                 *count.entry(message).or_insert(0) += 1;
        //             }
        //             for message in logs.clone().into_iter() {
        //                 if let Some(ct) = count.remove(&message) {
        //                     annotated.push((ct, message));
        //                 }
        //             }
        //         }
        //     });
        //
        //     for (count, message) in annotated {
        //         Label::new(
        //             cx,
        //             &format!(
        //                 "({}) {:?} - {}: {}",
        //                 count, message.level, message.source, message.message
        //             ),
        //         );
        //     }
        // });
    });
}
