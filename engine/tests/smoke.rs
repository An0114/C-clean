//! engine 冒烟测试：仅验证空 lib crate 可被独立编译与正确调用。
//! 业务级测试（白名单匹配、删除入口）在 task-003 落地。

use engine::hello_engine;

#[test]
fn hello_engine_returns_marker_string() {
    assert_eq!(hello_engine(), "engine");
}
