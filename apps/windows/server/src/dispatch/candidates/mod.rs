//! 候选窗口输出：Router 只产出 [`Frame`] 与光标矩形，交给 [`CandidateSink`] 去画；帧没变就不重画。

mod sink;

use qingjian_platform::protocol::{Frame, ScreenRect, SessionId};

pub use self::sink::{CandidateSink, NoopSink, RenderSettings};
use super::Router;

impl Router {
    /// 空帧（且没有句末译文）收窗口；否则已知光标矩形就重绘；还没收到矩形（组句刚起）先不显示，免得在旧位置闪一下。
    /// 没在组句只剩译文时，组句矩形清了就摆在最近一次知道的光标处。
    pub(super) fn reconcile_candidates(&mut self, frame: &Frame) {
        let echo = self.echo_lines();
        if frame.is_empty() && echo.is_empty() {
            self.engine.note_displayed(std::iter::empty());
            self.hide_candidate_window();
        } else if let Some(rect) = self.last_rect.or_else(|| self.echo_rect()) {
            let unchanged = matches!(&self.last_shown, Some((f, e, r)) if f == frame && *e == echo && *r == rect);
            if !unchanged {
                // 词汇记录的「看到轮次」按真正显示的页算，与 macOS 壳对齐。
                self.engine.note_displayed(frame.candidates.items.iter());
                self.candidates
                    .show_with_echo(frame.clone(), echo.clone(), rect);
                self.last_shown = Some((frame.clone(), echo, rect));
            }
        }
    }

    pub(super) fn hide_candidate_window(&mut self) {
        self.last_rect = None;
        self.last_shown = None;
        self.candidates.hide();
    }

    pub(super) fn position_candidates(&mut self, session: SessionId, rect: ScreenRect) {
        if self.focused != Some(session) {
            return;
        }
        self.last_rect = Some(rect);
        self.note_echo_rect(rect);
        // 自绘窗吃未降级的帧（降级只作用于发给 DLL 的那份）
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
    }
}
