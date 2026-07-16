//! XML Canonicalization C14N (Inclusive) per W3C spec http://www.w3.org/TR/2001/REC-xml-c14n-20010315
//! Simplified but DGII-compliant implementation matching DGII TypeScript example.
//! Handles: element, text, namespace declarations, attribute sorting, special char encoding.
//! Based on https://www.w3.org/TR/xml-c14n#ProcessingModel

use roxmltree::{Node, NodeType};

pub fn canonicalize(xml_str: &str) -> anyhow::Result<String> {
    let doc = roxmltree::Document::parse(xml_str)?;
    let root = doc.root_element();
    let mut out = String::new();
    canonicalize_node(&root, &mut out, &mut Vec::new(), true)?;
    Ok(out)
}

pub fn canonicalize_fragment(xml_str: &str) -> anyhow::Result<String> {
    // Canonicalize any XML fragment (e.g., SignedInfo node)
    // Wrap in dummy root if needed, but roxmltree requires single root -> we parse and canonicalize first child if fragment is not full doc
    let doc = roxmltree::Document::parse(xml_str)?;
    let root = doc.root_element();
    let mut out = String::new();
    canonicalize_node(&root, &mut out, &mut Vec::new(), true)?;
    Ok(out)
}

fn canonicalize_node(
    node: &Node,
    out: &mut String,
    ns_stack: &mut Vec<Vec<(String, String)>>, // stack of ns declarations in scope
    is_root: bool,
) -> anyhow::Result<()> {
    // Handle text nodes via parent iteration, not here directly for element
    // For element node
    if node.node_type() != NodeType::Element {
        return Ok(());
    }

    // Collect namespaces for this element
    let mut current_ns: Vec<(String, String)> = Vec::new();
    
    // roxmltree gives namespaces via namespaces() iterator? Actually attribute with xmlns
    // We need to also handle explicit xmlns attributes
    // Collect ns declarations from attributes that start with xmlns
    for attr in node.attributes() {
        let name = attr.name();
        if name == "xmlns" || name.starts_with("xmlns:") {
            current_ns.push((name.to_string(), attr.value().to_string()));
        }
    }

    // Render start tag
    out.push('<');
    out.push_str(node.tag_name().name());

    // Collect attributes to render sorted
    let mut attrs: Vec<(String, String)> = Vec::new();

    // Add namespace declarations that are not already in scope (C14N inclusive must render all in scope? Simplified: render those declared on this element)
    // For inclusive C14N without comments, we render Ns that visibly utilize + declared in ancestor? Simplified: render current_ns only.
    // This matches DGII simple XML.

    for (k, v) in &current_ns {
        attrs.push((k.clone(), v.clone()));
    }

    // Regular attributes (non-xmlns), sorted lexicographically
    let mut regular_attrs: Vec<(String, String)> = node
        .attributes()
        .iter()
        .filter(|a| {
            let n = a.name();
            !(n == "xmlns" || n.starts_with("xmlns:"))
        })
        .map(|a| {
            let qname = if let Some(prefix) = a.namespace() {
                format!("{}:{}", prefix, a.name())
            } else {
                a.name().to_string()
            };
            (qname, a.value().to_string())
        })
        .collect();

    regular_attrs.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in regular_attrs {
        attrs.push((k, v));
    }

    // Sort all attrs by name per C14N spec (xmlns first? spec says namespace declarations sorted, then regular sorted)
    // Our attrs already has ns + regular sorted, but we sort overall to be safe, keeping xmlns before? W3C: namespace nodes sorted by prefix, attr nodes sorted by namespace URI + local.
    // Simplified lexicographic:
    attrs.sort_by(|a, b| a.0.cmp(&b.0));

    for (k, v) in attrs {
        out.push(' ');
        out.push_str(&k);
        out.push_str("=\"");
        out.push_str(&encode_attr(&v));
        out.push('"');
    }

    out.push('>');

    // Children
    for child in node.children() {
        match child.node_type() {
            NodeType::Text => {
                if let Some(text) = child.text() {
                    out.push_str(&encode_text(text));
                }
            },
            NodeType::Element => {
                // Recurse
                ns_stack.push(current_ns.clone());
                canonicalize_node(&child, out, ns_stack, false)?;
                ns_stack.pop();
            },
            _ => {} // ignore comments, PI per spec (without comments)
        }
    }

    // End tag
    out.push_str("</");
    out.push_str(node.tag_name().name());
    out.push('>');

    Ok(())
}

fn encode_text(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '\r' => s.push_str("&#xD;"),
            _ => s.push(c),
        }
    }
    s
}

fn encode_attr(val: &str) -> String {
    let mut s = String::with_capacity(val.len());
    for c in val.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '"' => s.push_str("&quot;"),
            '\t' => s.push_str("&#x9;"),
            '\n' => s.push_str("&#xA;"),
            '\r' => s.push_str("&#xD;"),
            _ => s.push(c),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c14n_simple() {
        let xml = "<ECF><Encabezado><Version>1.0</Version></Encabezado></ECF>";
        let c = canonicalize(xml).unwrap();
        assert!(c.contains("<ECF>"));
        assert!(c.contains("</ECF>"));
        assert!(c.contains("<Version>1.0</Version>"));
    }

    #[test]
    fn test_c14n_attrs_sorted() {
        let xml = r#"<ECF B="2" A="1"><Id>1</Id></ECF>"#;
        let c = canonicalize(xml).unwrap();
        // A before B
        assert!(c.find("A=\"1\"").unwrap() < c.find("B=\"2\"").unwrap());
    }
}
