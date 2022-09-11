use dioxus::core as dioxus_core;
use dioxus_native_core;
use dioxus_native_core::state::*;
use dioxus_native_core_macro::{sorted_str_slice, State};

mod layout;
pub use layout::StretchLayout;
mod focus;
pub use focus::{Focus, FocusLevel, FocusState};

#[derive(Clone, PartialEq, Default, State, Debug)]
pub struct NodeState {
    // #[node_dep_state()]
    // pub(crate) mouse_effected: crate::mouse::MouseEffected,
    #[child_dep_state(layout, Rc<RefCell<Stretch>>)]
    pub layout: StretchLayout,
    // #[state]
    // pub style: crate::style::Style,
    #[node_dep_state()]
    pub focus: Focus,
    pub focused: bool,
    #[node_dep_state()]
    pub prevent_default: PreventDefault,
}

#[derive(PartialEq, Debug, Clone)]
pub enum PreventDefault {
    Focus,
    KeyPress,
    KeyRelease,
    KeyDown,
    KeyUp,
    MouseDown,
    Click,
    MouseEnter,
    MouseLeave,
    MouseOut,
    Unknown,
    MouseOver,
    ContextMenu,
    Wheel,
    MouseUp,
}

impl Default for PreventDefault {
    fn default() -> Self {
        PreventDefault::Unknown
    }
}

impl NodeDepState<()> for PreventDefault {
    type Ctx = ();

    const NODE_MASK: dioxus_native_core::node_ref::NodeMask =
        dioxus_native_core::node_ref::NodeMask::new_with_attrs(
            dioxus_native_core::node_ref::AttributeMask::Static(&sorted_str_slice!([
                "dioxus-prevent-default"
            ])),
        );

    fn reduce(
        &mut self,
        node: dioxus_native_core::node_ref::NodeView,
        _sibling: (),
        _ctx: &Self::Ctx,
    ) -> bool {
        let new = match node
            .attributes()
            .find(|a| a.name == "dioxus-prevent-default")
            .and_then(|a| a.value.as_text())
        {
            Some("onfocus") => PreventDefault::Focus,
            Some("onkeypress") => PreventDefault::KeyPress,
            Some("onkeyrelease") => PreventDefault::KeyRelease,
            Some("onkeydown") => PreventDefault::KeyDown,
            Some("onkeyup") => PreventDefault::KeyUp,
            Some("onclick") => PreventDefault::Click,
            Some("onmousedown") => PreventDefault::MouseDown,
            Some("onmouseup") => PreventDefault::MouseUp,
            Some("onmouseenter") => PreventDefault::MouseEnter,
            Some("onmouseover") => PreventDefault::MouseOver,
            Some("onmouseleave") => PreventDefault::MouseLeave,
            Some("onmouseout") => PreventDefault::MouseOut,
            Some("onwheel") => PreventDefault::Wheel,
            Some("oncontextmenu") => PreventDefault::ContextMenu,
            _ => return false,
        };
        if new == *self {
            false
        } else {
            *self = new;
            true
        }
    }
}
