//! 句末译文：一句话打完就把这句整句译成学习语言，在候选窗口里停留几秒，只看不上屏。
//!
//! 「打完」有三种：上屏的文字以 。？！ 结尾、没在组句时按回车（聊天框里就是发送）、上屏后停顿 [`PAUSE`] 没再按键。
//! 走自己的一条云端通道（与组句联想分开的 [`CloudPredictor`]）：打完紧接着敲下一句时，组句联想的新请求
//! 不会把这条译文当过期结果丢掉。一发出就占一行「翻译中…」，译文到了原地换掉；每段各占一行、
//! 各自停留 `[general] sentence_translation_seconds`（缺省 20 秒），旧的在上；接着打字时这些行
//! 接在候选下方，期间按键不收。只画在 Server 自绘的窗口里，不进发给 DLL 的帧。

use std::time::{Duration, Instant};

use qingjian_core::{Language, PredictionKind, PredictionRequest, Predictor};
use qingjian_platform::protocol::{KeyEvent, KeyOutcome, ScreenRect};
use qingjian_predict::{CloudPredictor, PredictConfig};

use super::Router;
use super::key::{BACK, RETURN, is_navigation};

/// 「翻译中…」最多等多久；云端没回（超时、出错）就收掉这一行。
const WAIT_FOR: Duration = Duration::from_secs(10);

/// 等译文时这一行显示的字。
const WAITING: &str = "翻译中…";

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
#[derive(Default)]
pub(super) struct Echo {
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

    /// 最近一次知道的光标矩形；没在组句时单独成窗就摆在这里。
    rect: Option<ScreenRect>,
}

/// 一段：对应的请求、译文（`None` 是还在等）与收起的时刻（等译文时是超时时刻）。
struct Line {
    sequence: u64,

    text: Option<String>,

    until: Instant,
}

impl Router {
    /// 按 `[predict]` 接 / 换句末译文的云端通道；启动与配置热加载时调。
    pub fn attach_echo(&mut self, predict: &PredictConfig) {
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

    /// 记下光标矩形：句末译文单独成窗时摆在这里。
    pub(super) fn note_echo_rect(&mut self, rect: ScreenRect) {
        self.echo.rect = Some(rect);
    }

    /// 换了会话：上一句没打完的半句作废。
    pub(super) fn clear_echo_buffer(&mut self) {
        self.echo.buffer.clear();
    }

    /// 一次按键处理完：把上屏的文字攒进这句话，句末就发翻译。`composing` 是按键前在不在组句。
    pub(super) fn note_echo_key(
        &mut self,
        event: &KeyEvent,
        commit: Option<&str>,
        outcome: KeyOutcome,
        composing: bool,
    ) {
        if self.echo.predictor.is_none() {
            return;
        }
        self.echo.last_key = Some(Instant::now());
        if let Some(text) = commit {
            self.echo.buffer.push_str(text);
            if text.trim_end().ends_with(SENTENCE_ENDS) {
                self.translate_sentence();
            }
            return;
        }
        if outcome != KeyOutcome::Passthrough || composing {
            return;
        }
        // 没在组句、交给应用的键：回车算一句结束，退格删掉一个字，挪光标说明去别处改了
        match event.virtual_key {
            RETURN => self.translate_sentence(),
            BACK => {
                self.echo.buffer.pop();
            }
            key if is_navigation(key) => self.echo.buffer.clear(),
            _ => {
                if let Some(c) = event.character.filter(|c| !c.is_control()) {
                    self.echo.buffer.push(c);
                    if SENTENCE_ENDS.contains(&c) {
                        self.translate_sentence();
                    }
                }
            }
        }
    }

    /// 把攒下的这段发去翻译；汉字太少、私密输入框里不发（攒的照样清掉）。
    fn translate_sentence(&mut self) {
        let text = std::mem::take(&mut self.echo.buffer);
        let text = text.trim();
        if han_count(text) < MIN_HAN || self.engine.is_private() {
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
    }

    /// 借 DLL 的轮询 / 同步节拍：停顿够了就发翻译、收译文、到点收起。返回显示内容有没有变。
    pub(super) fn tick_echo(&mut self) -> bool {
        let now = Instant::now();
        let paused = self
            .echo
            .last_key
            .is_some_and(|at| now.duration_since(at) >= PAUSE);
        let before = self.echo.lines.len();
        if paused && self.engine.composition().is_empty() && !self.echo.buffer.trim().is_empty() {
            self.translate_sentence();
        }
        let mut changed = self.echo.lines.len() != before;
        let before = self.echo.lines.len();
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
                        line.until = now + self.config.echo_show_for;
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
        changed || self.echo.lines.len() != before
    }

    /// 同步节拍（没在组句时 DLL 每 320 ms 来一次）：有变化、或还有译文要显示（窗口可能被应用收过）就对一次自绘窗。
    pub(super) fn refresh_echo(&mut self) {
        if self.tick_echo() || !self.echo.lines.is_empty() {
            let frame = self.self_drawn_frame();
            self.reconcile_candidates(&frame);
        }
    }

    /// 要接在候选下方的译文行；翻译选中文字时不掺和。
    pub(super) fn echo_lines(&self) -> Vec<String> {
        if self.translation.is_some() {
            return Vec::new();
        }
        self.echo
            .lines
            .iter()
            .map(|line| line.text.clone().unwrap_or_else(|| WAITING.to_owned()))
            .collect()
    }

    /// 单独成窗时用的位置：组句的矩形没了（上屏后会清）就用最近一次知道的。
    pub(super) fn echo_rect(&self) -> Option<ScreenRect> {
        if self.echo.lines.is_empty() {
            return None;
        }
        self.echo.rect
    }
}

fn han_count(text: &str) -> usize {
    text.chars()
        .filter(|c| {
            matches!(*c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
        })
        .count()
}
