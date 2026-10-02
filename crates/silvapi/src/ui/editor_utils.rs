use gpui_kit::base::input::{Copy, Cut, Paste, SelectAll};
use gpui_kit::component::native_menu::NativeMenu;
use gpui_kit::{App, Window};

pub fn editor_context_menu(menu: NativeMenu, _window: &mut Window, _cx: &mut App) -> NativeMenu {
    menu.menu("Cut", Box::new(Cut))
        .menu("Copy", Box::new(Copy))
        .menu("Paste", Box::new(Paste))
        .separator()
        .menu("Select All", Box::new(SelectAll))
}

pub fn readonly_editor_context_menu(
    menu: NativeMenu,
    _window: &mut Window,
    _cx: &mut App,
) -> NativeMenu {
    menu.menu("Copy", Box::new(Copy))
        .separator()
        .menu("Select All", Box::new(SelectAll))
}
