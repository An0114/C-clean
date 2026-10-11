//! 独立提权 exe 占位 crate（M0）。
//!
//! 设计约定：本二进制只会以「按需 UAC 提权、完成即退出」的短生命周期方式，
//! 被主进程通过命名管道拉起，绝不常驻。普通用户直接运行它没有任何意义，
//! 因此占位实现只打印一行提示并以非 0 退出码退出，明示「不应直接运行」。

fn main() {
    eprintln!("elevation stub, not for direct run");
    std::process::exit(1);
}
