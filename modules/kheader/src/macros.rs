pub use linkme;
pub use linkme::distributed_slice as linker_define;
pub use linkme::distributed_slice as linker_use;

/*
    item：条目，例如函数、结构、模块等
    block：代码块
    stmt：语句
    pat：模式
    expr：表达式
    ty：类型
    ident：标识符
    path：路径，例如 foo、 ::std::mem::replace, transmute::<_, int>, …
    meta：元信息条目，例如 #[…]和 #![rust macro…] 属性
    tt：词条树
*/

pub macro module_use($name:ty) {
    pub use $name;
}

pub macro link_define(
    $($args:item)*
) {
    $(
        #[linker_define]
        #[linkme(crate = $crate::macros::linkme)]
        $args
    )*
}

