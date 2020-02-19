use std::io::{self, Write};

use yarte_html::{
    interface::QualName,
    serializer::{HtmlSerializer, SerializerOpt},
    tree_builder::is_marquee,
};

use crate::sink::{ParseAttribute, ParseElement, ParseNodeId, Sink};

pub fn serialize<Wr>(writer: Wr, node: Tree, opts: SerializerOpt) -> io::Result<()>
where
    Wr: Write,
{
    let mut ser = HtmlSerializer::new(writer, opts);
    node.serialize(&mut ser)
}

#[derive(Debug)]
pub enum TreeElement {
    Node {
        name: QualName,
        attrs: Vec<ParseAttribute>,
        children: Vec<TreeElement>,
    },
    Text(String),
    DocType,
}

pub struct Tree {
    nodes: Vec<TreeElement>,
}

impl From<Sink> for Tree {
    fn from(sink: Sink) -> Tree {
        use ParseElement::*;

        let nodes = match sink.nodes.values().next() {
            Some(Document(children)) => {
                let mut tree = vec![TreeElement::DocType];
                tree.extend(get_children(children, &sink));
                tree
            }
            Some(Node {
                name,
                attrs,
                children,
                ..
            }) => {
                if is_marquee(name) {
                    get_children(children, &sink)
                } else {
                    vec![TreeElement::Node {
                        name: name.clone(),
                        attrs: attrs.to_vec(),
                        children: get_children(children, &sink),
                    }]
                }
            }
            Some(Text(s)) => vec![TreeElement::Text(s.clone())],
            None => vec![],
        };

        Tree { nodes }
    }
}

fn get_children(children: &[ParseNodeId], sink: &Sink) -> Vec<TreeElement> {
    use ParseElement::*;
    let mut tree = vec![];
    for child in children {
        match sink.nodes.get(child).expect("Child") {
            Text(s) => {
                if let Some(TreeElement::Text(last)) = tree.last_mut() {
                    last.push_str(s);
                } else {
                    tree.push(TreeElement::Text(s.clone()))
                }
            }
            Node {
                name,
                attrs,
                children,
                ..
            } => tree.push(TreeElement::Node {
                name: name.clone(),
                attrs: attrs.to_vec(),
                children: get_children(children, sink),
            }),
            _ => panic!("Expect document in root"),
        }
    }

    tree
}

impl Tree {
    pub fn serialize<W: Write>(self, serializer: &mut HtmlSerializer<W>) -> io::Result<()> {
        _serialize(self.nodes, serializer, None)
    }
}

fn _serialize<W: Write>(
    nodes: Vec<TreeElement>,
    serializer: &mut HtmlSerializer<W>,
    parent: Option<QualName>,
) -> io::Result<()> {
    use TreeElement::*;
    for node in nodes {
        match node {
            Node {
                children,
                name,
                attrs,
            } => {
                serializer.start_elem(
                    name.clone(),
                    attrs.iter().map(|x| (&x.name, x.value.as_str())),
                )?;
                _serialize(children, serializer, Some(name.clone()))?;
                serializer.end_elem(name.clone())?
            }
            Text(ref s) => serializer.write_text(s)?,
            DocType => serializer.write_doctype("html")?,
        }
    }
    serializer.end(parent)
}
