# GPUI Conventions & Core Architecture

This guide details the architectural conventions, mental model, and best practices for building responsive, maintainable desktop applications with **GPUI** and **`gpui-kit`**.

---

## 1. The GPUI Mental Model

GPUI is a GPU-accelerated UI framework developed in Rust. Unlike Electron (which uses HTML/DOM/Chromium) or traditional retained widget toolkits (like Qt/GTK), GPUI blends the ergonomics of **declarative immediate-mode element trees** with **retained Entity state**.

```mermaid
graph LR
    EntityState["Entity State<br/>(Model / Business Data)"] -->|cx.notify()| ReRender["Render Pass<br/>(fn render(&mut self))"]
    ReRender --> ElementTree["Declarative Element Tree<br/>(div(), flex, components)"]
    ElementTree --> GPULayout["GPU Text Shaping & Layout<br/>(Taffy + Blade/Metal/WGPU)"]
```

---

## 2. Entities (`Entity<T>`) & State Management

An `Entity<T>` is a pointer to state managed by the GPUI runtime within an `App` or `Window` context:

### Creating an Entity
```rust
let my_state: Entity<MyState> = cx.new(|cx| MyState::new());
```

### Reading State
```rust
// Immutably read state within a render or callback:
let value = my_entity.read(cx).some_field.clone();
```

### Mutating State & Triggering Re-renders
```rust
// Mutating triggers subscribers or views observing this entity:
my_entity.update(cx, |state, cx| {
    state.active_request_id = Some("req_123".into());
    cx.notify(); // Notifies GPUI to schedule a frame re-render
});
```

### The `EventEmitter` Pattern
Entities can emit strongly typed events that views subscribe to:
```rust
impl EventEmitter<AppEvent> for AppState {}

// In a panel:
let _sub = cx.subscribe(&app_state, |this, _emitter, event, cx| {
    match event {
        AppEvent::RequestSelected(id) => this.load_request(id, cx),
        AppEvent::ResponseReceived => cx.notify(),
    }
});
```

---

## 3. The `Render` Trait & Declarative Elements

Every view implements the `Render` trait. It returns a composable element tree on every frame:

```rust
impl Render for RequestPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_url_bar(window, cx))
            .child(self.render_tabs(window, cx))
    }
}
```

### Core Element Conventions
1. **Never build deep nested wrapper divs**:
   - Instead of `div().child(div().child(div().child(...)))`, compose directly using flex containers: `h_flex().gap_2().child(...)`.
2. **Use Semantic Flex Containers**:
   - `h_flex()` = horizontal flex container (`flex-row`, centered items).
   - `v_flex()` = vertical flex container (`flex-col`).
3. **Chain Layout Modifiers Directly**:
   - Sizing: `.size_full()`, `.w_full()`, `.h_full()`, `.flex_1()`, `.min_w_0()`.
   - Spacing: `.p_2()` (padding), `.px_4()`, `.py_2()`, `.gap_2()`, `.gap_4()`.
   - Borders & Corners: `.border_1()`, `.border_color(...)`, `.rounded_md()`.

---

## 4. Theme System & Styling

Always use `cx.theme()` for colors and typography rather than hardcoding RGB or hex values.

### Active Theme Tokens
```rust
let theme = cx.theme();

div()
    .bg(theme.background)
    .text_color(theme.foreground)
    .border_color(theme.border)
```

Common tokens:
- `theme.background`: App background.
- `theme.foreground`: Primary text color.
- `theme.muted_foreground`: Subdued/secondary labels and placeholders.
- `theme.primary`: Accent color (buttons, active indicators).
- `theme.primary_foreground`: Text on accent buttons.
- `theme.border`: Border lines and divider hairlines.
- `theme.destructive`: Error and deletion states.

---

## 5. Event Handlers & Listeners

GPUI uses `cx.listener(...)` to wire UI interactions back to view methods:

```rust
Button::new("send_btn")
    .primary()
    .label("Send")
    .on_click(cx.listener(|this, _event, window, cx| {
        this.execute_active_request(window, cx);
    }))
```

- `this`: Mutable reference to the current view component.
- `_event`: Mouse or keyboard event payload.
- `window`: Active window handle for focus, sizing, and popovers.
- `cx`: Context for notifications, entity updates, and subscriptions.

---

## 6. High-Level `gpui-kit` Components vs Base Primitives

Always prioritize `gpui-kit::component` widgets over raw HTML-like divs:

| Use Case | ❌ Don't Hand-Roll | ✅ Use gpui-kit Component |
|---|---|---|
| Split Panes | Custom mouse-drag math & rect splitting | `h_resizable("id")` / `v_resizable("id")` |
| Navigation Tabs | Custom divs with click toggles & border styling | `TabBar::new("tabs").child(Tab::new(...))` |
| Text Input | Raw text rendering with cursor logic | `Input::new(&input_state)` |
| Multi-Line Code | Manual line wrapping & scroll tracking | `Textarea::new(&textarea_state)` |
| Dropdown Select | Custom popover layers with arrow keys | `Select::new(&select_state)` |
| Action Buttons | Divs with `.on_mouse_down` and manual hover colors | `Button::new("id").primary().label("Send")` |
| Tree / Outline | Nested indent calculations | `tree::Tree` |
| Virtual Grids | Iterating thousands of divs | `table::DataTable` with `TableState` |

---

## 7. Tsoding-Style Top-Down Architecture

When architecting a view:
1. **Define High-Level API First**: Name the conceptual parts (`TopBar`, `Sidebar`, `RequestPanel`, `ResponsePanel`).
2. **Build the Shell / Skeleton**: Connect the top-level layout with dummy/placeholder data before implementing internal details.
3. **Fill Panel by Panel**: Complete each panel's minimal working logic without premature abstractions or dead configuration flags.
4. **Iterate & Verify**: Keep the code lean, understandable in one reading, and compiling cleanly at each step.
