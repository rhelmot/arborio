use vizia::*;

use crate::app_state::AppState;

pub fn build_logs(cx: &mut Context) {
    ScrollView::new(cx, 0.0, 100.0, false, true, |cx| {
        List::new(cx, AppState::logs, |cx, _, item| {
            // log is append-only so we can safely not bind to the item lens
            let message = item.get(cx).take();
            Label::new(
                cx,
                &format!(
                    "{:?} - {}: {}",
                    message.level, message.source, message.message
                ),
            );
        });
    });
}
