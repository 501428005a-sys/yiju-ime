mod channel;

pub use channel::UpdateChannel;

use serde::{Deserialize, Serialize};

/// 配置文件 `[update]` 分节：检查更新。
///
/// 开着时每天读一次版本索引，有新版就在菜单与设置的「关于」页提示；请求不带任何标识，不下载也不安装。
/// 译句还没有自己签名的版本索引，缺省关闭。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// 开着就每天检查一次。
    pub check: bool,

    /// 看哪个渠道的版本。
    pub channel: UpdateChannel,
}
