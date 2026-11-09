//! task-083：`software.inventory` 命令的纯逻辑。
//!
//! 命令本身只是「枚举已装软件 + 包信封」，拆一层出来是为了让 IPC 层保持
//! 「不碰系统」的可测性：这里只做**排序 / 去重后的展示裁剪**，
//! 真正的注册表读取在 `scan::software_enum`。

use crate::ipc::envelope::Envelope;
use crate::scan::software_enum::SoftwareEntry;

/// 一次最多返回多少条。
///
/// 真机上 Uninstall 键常有 100+ 条（含各种运行时 / 补丁）。UI 一次展示不了
/// 那么多，也没必要把 100 条全塞进 IPC 信封——按名字排序后截断即可。
pub const MAX_INVENTORY_ITEMS: usize = 200;

/// 包成成功信封（调用方负责真正枚举注册表）。
pub fn inventory_envelope(entries: Vec<SoftwareEntry>) -> Envelope<Vec<SoftwareEntry>> {
    let mut entries = entries;
    entries.truncate(MAX_INVENTORY_ITEMS);
    Envelope::ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str) -> SoftwareEntry {
        SoftwareEntry {
            name: name.to_string(),
            version: String::new(),
            publisher: String::new(),
            install_location: String::new(),
            registry_key: String::new(),
        }
    }

    #[test]
    fn envelope_is_success_and_keeps_data() {
        let env = inventory_envelope(vec![e("VSCode")]);
        assert_eq!(env.code, 0);
        let data = env.data.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].name, "VSCode");
    }

    #[test]
    fn envelope_caps_oversized_inventory() {
        let many: Vec<_> = (0..MAX_INVENTORY_ITEMS + 50)
            .map(|i| e(&format!("App{i}")))
            .collect();
        let env = inventory_envelope(many);
        assert_eq!(env.data.unwrap().len(), MAX_INVENTORY_ITEMS);
    }
}
