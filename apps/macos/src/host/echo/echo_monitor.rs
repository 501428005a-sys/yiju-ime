//! 句末译文的定时器：攒着半句（等停顿）或有译文在显示时跑，没事可做就停，平时不占 CPU。
//!
//! Windows 借 DLL 的轮询节拍，macOS 的 IMK 没有这种节拍，只能自己起一个 NSTimer。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 节拍间隔：停顿判定是 1 秒，晚一两百毫秒无妨。
const TICK_INTERVAL: f64 = 0.1;

pub struct EchoMonitor {
    /// 定时器；没事可做时为 `None`。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl EchoMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self { timer: None, mtm }
    }

    /// 开始（或继续）跑。
    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = EchoTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                TICK_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct EchoTicker;

    impl EchoTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.tick_echo());
        }
    }

    unsafe impl NSObjectProtocol for EchoTicker {}
);

impl EchoTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
