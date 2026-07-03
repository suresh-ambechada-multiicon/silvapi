use gpui::*;
use gpui_component::ActiveTheme;
use scraper::{Html, Node};

#[derive(Clone, Debug)]
pub enum HtmlNode {
    Element {
        tag: String,
        href: Option<String>,
        src: Option<String>,
        children: Vec<HtmlNode>,
    },
    Text(String),
}

pub fn parse_html(html: &str) -> Vec<HtmlNode> {
    let document = Html::parse_document(html);
    let mut count = 0;

    let mut root_children = Vec::new();
    for child in document.tree.root().children() {
        if let Some(n) = convert_node(child, &mut count, 2000) {
            root_children.push(n);
        }
        if count >= 2000 {
            root_children.push(HtmlNode::Text(
                "\n[... HTML Preview truncated due to size ...]".into(),
            ));
            break;
        }
    }

    root_children
}

fn convert_node(node: ego_tree::NodeRef<Node>, count: &mut usize, max: usize) -> Option<HtmlNode> {
    if *count >= max {
        return None;
    }
    *count += 1;

    match node.value() {
        Node::Element(el) => {
            let tag = el.name().to_string();
            // Ignore script and style
            if tag == "script" || tag == "style" || tag == "svg" || tag == "head" {
                return None;
            }

            let mut children = Vec::new();
            for child in node.children() {
                if let Some(c) = convert_node(child, count, max) {
                    children.push(c);
                }
                if *count >= max {
                    break;
                }
            }

            let href = el.attr("href").map(|s| s.to_string());
            let src = el.attr("src").map(|s| s.to_string());

            Some(HtmlNode::Element {
                tag,
                href,
                src,
                children,
            })
        }
        Node::Text(text) => {
            let t = text.text.trim();
            if t.is_empty() {
                None
            } else {
                Some(HtmlNode::Text(t.to_string()))
            }
        }
        _ => None,
    }
}

pub fn render_node(node: &HtmlNode, cx: &App) -> AnyElement {
    match node {
        HtmlNode::Text(t) => div()
            .child(t.clone())
            .text_color(cx.theme().foreground)
            .into_any_element(),
        HtmlNode::Element {
            tag,
            href: _,
            src: _,
            children,
        } => {
            let mut el = div().w_full().flex().flex_wrap();

            match tag.as_str() {
                "h1" => el = el.text_3xl().font_weight(FontWeight::BOLD).py_2(),
                "h2" => el = el.text_2xl().font_weight(FontWeight::BOLD).py_2(),
                "h3" => el = el.text_xl().font_weight(FontWeight::BOLD).py_1(),
                "p" => el = el.py_1().w_full(),
                "b" | "strong" => el = el.font_weight(FontWeight::BOLD),
                "i" | "em" => el = el.italic(),
                "div" | "body" | "html" | "main" | "section" | "article" => {
                    el = el.w_full().flex_col()
                }
                "span" | "a" => {}
                _ => el = el.w_full(),
            }

            let mut rendered_children = Vec::new();
            for c in children {
                rendered_children.push(render_node(c, cx));
            }

            el.children(rendered_children).into_any_element()
        }
    }
}
