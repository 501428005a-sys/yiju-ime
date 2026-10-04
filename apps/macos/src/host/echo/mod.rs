//! 句末译文（整句翻译）：一句话打完就把这句整句译成学习语言，接在候选窗口下方停留几秒，只看不上屏。
//!
//! 与 Windows Server 的 `dispatch/echo` 同一套规则，按 macOS 壳的结构搬过来：
//! 「打完」有三种：上屏的文字以 。？！ 结尾、没在组句时按回车（聊天框里就是发送）、上屏后停顿 [`PAUSE`] 没再按键。
//! 走自己的一条云端通道（与组句联想分开的 [`CloudPredictor`]）：打完紧接着敲下一句时，组句联想的新请求
//! 不会把这条译文当过期结果丢掉。一发出就占一行「翻译中…」，译文到了原地换掉；每段各占一行、
//! 各自停留 `[general] sentence_translation_seconds`（缺省 20 秒），旧的在上；接着打字时这些行
//! 接在候选下方（序号位写「译」），不参与高亮与选词。停顿判定、收译文与到点收起靠 [`EchoMonitor`] 的定时器。

mod echo_monitor;

pub(super) use echo_monitor::EchoMonitor;

use std::time::{Duration, Instant};

use qingjian_core::{PredictionKind, PredictionRequest, Predictor};

use super::*;
use crate::imk::secure_input;

/// 「翻译中…」最多等多久；云端没回（超时、出错）就收掉这一行。
const WAIT_FOR: Duration = Duration::from_secs(10);

/// 等译文时这一行显示的字。
const WAITING: &str = "翻译中…";

/// 序号位写的字，与 Windows 一致。
const LABEL: &str = "译";

/// 上屏后停顿多久没按键就当这句打完了。
const PAUSE: Duration = Duration::from_secs(1);

/// 同时最多显示几段译文，再多就挤掉最旧的。
const MAX_LINES: usize = 4;

/// 一段里至少几个汉字才翻译：单个字、纯英文 / 数字不值得问云端。
const MIN_HAN: usize = 2;

/// 一次最多翻译多少字（与「翻译选中文字」的上限一致），超出只取最后这么多。
const MAX_CHARS: usize = 500;

/// 句末标点：上屏的文字以它们结尾就算一句话打完。
const SENTENCE_ENDS: &[char] = &['。', '！', '？', '…', '!', '?', '.'];

/// 句末译文的进行态。
pub struct Echo {
    /// 专用的云端通道；云联想关着或缺密钥时为 `None`，整个功能不工作。
    predictor: Option<CloudPredictor>,

    /// 请求序号计数器。
    sequence: u64,

    /// 从上一段结束起上屏过的文字（分几次选词也连起来）。
    buffer: String,

    /// 最近一次按键的时刻；停顿判定从这里算。
    last_key: Option<Instant>,

    /// 正在显示的各段（等译文的显示「翻译中…」），按发出顺序，旧的在前。
    lines: Vec<Line>,

    /// 译文到了之后停留多久（`[general] sentence_translation_seconds`）。
    show_for: Duration,

    /// 停顿判定 / 收译文 / 到点收起的定时器，有事可做时才跑。
    monitor: EchoMonitor,
}

/// 一段：对应的请求、译文（`None` 是还在等）与收起的时刻（等译文时是超时时刻）。
struct Line {
    sequence: u64,

    text: Option<String>,

    until: Instant,
}

impl Echo {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            predictor: None,
            sequence: 0,
            buffer: String::new(),
            last_key: None,
            lines: Vec::new(),
            show_for: Duration::from_secs(20),
            monitor: EchoMonitor::new(mtm),
        }
    }

    /// 还有没有事要等：攒着半句（等停顿）或有行在显示。
    fn busy(&self) -> bool {
        !self.buffer.trim().is_empty() || !self.lines.is_empty()
    }
}

impl Host {
    /// 按 `[predict]` 接 / 换句末译文的云端通道；`[predict]` 变了（与组句联想的 Predictor 同时重建）时调。
    pub(super) fn attach_echo(&mut self, predict: &PredictConfig) {
        self.echo.predictor = if predict.enabled {
            match CloudPredictor::new(predict) {
                Ok(predictor) => Some(predictor),
                Err(error) => {
                    tracing::warn!(%error, "句末译文未启用");
                    None
                }
            }
        } else {
            None
        };
        // 换了通道，在等的那些不会再有回音
        self.echo.lines.retain(|line| line.text.is_some());
    }

    /// 译文到了之后停留多久（`[general] sentence_translation_seconds`）；配置套用时调。
    pub(super) fn set_echo_duration(&mut self, show_for: Duration) {
        self.echo.show_for = show_for;
    }

    /// 每个按下的键都记一下时刻：停顿多久从最后一键算。
    pub fn note_echo_key(&mut self) {
        if self.echo.predictor.is_some() {
            self.echo.last_key = Some(Instant::now());
        }
    }

    /// 文字上屏了（选词、标点、原样上屏都算）：攒进这句话，句末就发翻译。
    pub fn note_echo_commit(&mut self, text: &str) {
        if self.echo.predictor.is_none() {
            return;
        }
        self.echo.buffer.push_str(text);
        if text.trim_end().ends_with(SENTENCE_ENDS) {
            self.translate_sentence();
        }
        self.watch_echo();
    }

    /// 没在组句、直接交给应用的字符：回车算一句结束，其他可见字符也攒进这句话。
    pub fn note_passthrough(&mut self, c: char) {
        self.engine.note_passthrough(c);
        if self.echo.predictor.is_none() {
            return;
        }
        if c == '\n' {
            self.translate_sentence();
        } else if !c.is_control() {
            self.echo.buffer.push(c);
            if SENTENCE_ENDS.contains(&c) {
                self.translate_sentence();
            }
        }
        self.watch_echo();
    }

    /// 没在组句时退格：删的是应用里刚打的字，攒的这句话也去掉一个字。
    pub fn note_echo_backspace(&mut self) {
        self.echo.buffer.pop();
    }

    /// 挪了光标、按词 / 按行删、换了应用：去别处改了，没打完的半句作废。
    pub fn clear_echo_buffer(&mut self) {
        self.echo.buffer.clear();
    }

    /// 切走输入法：半句作废、正在显示的译文也收掉（窗口已被收起，不能留着到点再冒出来）。
    pub fn clear_echo(&mut self) {
        self.echo.buffer.clear();
        self.echo.lines.clear();
        self.echo.last_key = None;
        self.echo.monitor.stop();
    }

    /// 把攒下的这段发去翻译；汉字太少、Secure Input（密码框）里不发（攒的照样清掉）。
    fn translate_sentence(&mut self) {
        let text = std::mem::take(&mut self.echo.buffer);
        let text = text.trim();
        if han_count(text) < MIN_HAN || secure_input::enabled() {
            return;
        }
        let skip = text.chars().count().saturating_sub(MAX_CHARS);
        let text: String = text.chars().skip(skip).collect();
        // 原句以汉字为主，译成学习语言；没开学习语言（中文）时译成英文
        let target = match self.engine.learning_language() {
            Language::Chinese => Language::English,
            other => other,
        };
        let Some(predictor) = self.echo.predictor.as_mut() else {
            return;
        };
        self.echo.sequence += 1;
        predictor.submit(PredictionRequest {
            sequence: self.echo.sequence,
            kind: PredictionKind::Translate,
            before: String::new(),
            after: String::new(),
            pinyin: String::new(),
            letters: String::new(),
            syllables: 0,
            candidates: Vec::new(),
            guess: String::new(),
            max_items: 1,
            want_sentence: true,
            text: text.clone(),
            target_language: target.code().to_owned(),
        });
        // 先占一行「翻译中…」，译文到了原地换掉，几段的顺序就是打的顺序
        self.echo.lines.push(Line {
            sequence: self.echo.sequence,
            text: None,
            until: Instant::now() + WAIT_FOR,
        });
        tracing::debug!(chars = text.chars().count(), "句末译文：已发翻译请求");
        self.render_echo();
    }

    /// 有事要等就让定时器跑起来。
    fn watch_echo(&mut self) {
        if self.echo.busy() {
            self.echo.monitor.start();
        }
    }

    /// 定时器回调：停顿够了就发翻译、收译文、到点收起；显示内容变了就重画，没事可做就停表。
    pub fn tick_echo(&mut self) {
        let now = Instant::now();
        let paused = self
            .echo
            .last_key
            .is_some_and(|at| now.duration_since(at) >= PAUSE);
        if paused && self.engine.composition().is_empty() && !self.echo.buffer.trim().is_empty() {
            // 发出去时已经重画过
            self.translate_sentence();
        }
        let before = self.echo.lines.len();
        let mut changed = false;
        if let Some(predictor) = self.echo.predictor.as_mut() {
            while let Some(prediction) = predictor.poll() {
                let Some(index) =
                    self.echo.lines.iter().position(|line| {
                        line.sequence == prediction.sequence && line.text.is_none()
                    })
                else {
                    continue;
                };
                // 云端没给译文就收掉这一行
                match prediction.sentence {
                    Some(text) => {
                        let line = &mut self.echo.lines[index];
                        line.text = Some(text);
                        line.until = now + self.echo.show_for;
                    }
                    None => {
                        self.echo.lines.remove(index);
                    }
                }
                changed = true;
            }
        }
        self.echo.lines.retain(|line| now < line.until);
        let overflow = self.echo.lines.len().saturating_sub(MAX_LINES);
        self.echo.lines.drain(..overflow);
        if changed || self.echo.lines.len() != before {
            self.render_echo();
        }
        if !self.echo.busy() {
            self.echo.monitor.stop();
        }
    }

    /// 译文行变了：按当前会话重画（没在组句时窗口里只有译文行，没有译文行就收窗）。
    /// 翻译选中文字、提示正在显示时窗口归它们，不掺和。
    fn render_echo(&mut self) {
        if self.translation.is_none() && self.notice.is_none() {
            self.render();
        }
    }

    /// 要接在候选下方的译文行；翻译选中文字、提示正在显示时没有。
    pub(super) fn echo_rows(&self) -> Vec<Row> {
        if self.translation.is_some() || self.notice.is_some() {
            return Vec::new();
        }
        self.echo
            .lines
            .iter()
            .map(|line| Row {
                index: LABEL.to_owned(),
                text: line.text.clone().unwrap_or_else(|| WAITING.to_owned()),
                annotation: Vec::new(),
                cloud: false,
            })
            .collect()
    }
}

fn han_count(text: &str) -> usize {
    text.chars()
        .filter(|c| {
            matches!(*c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::han_count;

    #[test]
    fn counts_only_han_characters() {
        assert_eq!(han_count("明天开会，ok"), 4);
        assert_eq!(han_count("hello 123"), 0);
        assert_eq!(han_count("好"), 1);
    }
}
