//! Module to parse and write dot files for graphs.

// TODO:
//   * Add colors to different elements of the graph.

pub use super::cfg::*;
pub use super::esil::*;

// Remove after make_cfg() is removed.
use std::io::prelude::*;
use std::fs::File;

macro_rules! add_strings {
    ( $( $x: expr ),* ) => {
        {
            let mut s = String::new();
            $(
                s = format!("{}{}", s, $x);
             )*
                s
        }
    };
}

pub trait Dot {
    fn to_dot(&self) -> String;
}

impl Dot for CFG {
    fn to_dot(&self) -> String {
        let mut result = String::new();
        result = add_strings!(result, "digraph cfg {\n");
        // Graph configurations
        // Node configurations
        for node in self.g.raw_nodes().iter() {
            result = add_strings!(result, node.weight.to_dot());
        }
        // Connect BasicBlocks by edges.
        for edge in self.g.raw_edges() {
            let src_node = self.g.node_weight(edge.source()).unwrap();
            let dst_node = self.g.node_weight(edge.target()).unwrap();
            let mut direction = "forward";
            let mut color = "black";
            // TODO: Must be changed once edges get their own attributes.
            if edge.weight == 1 {
                direction = "back";
                color = "blue";
            }
            result = add_strings!(result, src_node.label, " -> ", dst_node.label,
                                  "[color=", color, " dir=", direction, "];\n");
        }
        add_strings!(result, "}")
    }
}

impl Dot for BasicBlock {
    fn to_dot(&self) -> String {
        let mut result = String::new();
        result = add_strings!(result, "<<table border=\"0\" cellborder=\"0\" cellpadding=\"1\">");
        for inst in &self.instructions {
            result = add_strings!(result, inst.to_dot());
        }
        result = add_strings!(result, "</table>>");
        add_strings!(self.label, "[label=", result, " shape=box];\n")
    }
}

impl Dot for Instruction {
    fn to_dot(&self) -> String {
        format!("<tr><td align=\"left\" cellspacing=\"1\"><font color=\"grey50\"
                point-size=\"9\">0x{:08x}:</font></td><td align=\"left\">{}</td></tr>", self.addr,
                self)
    }
}

// Dummy Function for test purposes. Will be removed later.
pub fn make_dot(g: CFG) {
    let mut dot_file = File::create("cfg.dot").ok().expect("Error. Cannot
                                                           create file!\n");
    dot_file.write_all(g.to_dot().as_bytes()).ok().expect("Error. Cannot write
                                                          file!\n");
    println!("[*] Dot file written!");
    println!("[*] Run `./scripts/genpng.sh cfg.dot` to generate the graph.");
}
