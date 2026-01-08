use taffy::prelude::*;
use crate::symbol_table::Value;

pub struct LayoutEngine {
    taffy: Taffy,
    root: Node,
}

impl LayoutEngine {
    pub fn new() -> Self {
        let mut taffy = Taffy::new();
        // default root
        let root = taffy.new_leaf(Style::default()).unwrap();
        Self { taffy, root }
    }

    pub fn compute_layout(&mut self, root_value: &Value, width: f32, height: f32) -> Vec<(f32, f32, f32, f32, Value)> {
        // Rebuild tree from Value
        self.taffy = Taffy::new();
        
        // Root container (Window)
        let root_style = Style {
            size: Size { width: Dimension::Points(width), height: Dimension::Points(height) },
            ..Default::default()
        };
        
        // Ensure root is created
        self.root = self.taffy.new_leaf(root_style).unwrap();
        
        // Recursively build tree
        let content_root = self.build_tree(root_value);
        self.taffy.add_child(self.root, content_root).unwrap();
        
        // Compute
        self.taffy.compute_layout(
            self.root,
            Size { width: AvailableSpace::Definite(width), height: AvailableSpace::Definite(height) }
        ).unwrap();
        
        // Collect results
        let mut results = Vec::new();
        // Collect from content_root, offset by root (0,0)
        self.collect_results(content_root, root_value, 0.0, 0.0, &mut results);
        
        results
    }
    
    fn build_tree(&mut self, value: &Value) -> Node {
        match value {
             Value::Component { style: _, ty: _, label: _, children, layout } => {
                 let mut flex_dir = FlexDirection::Column;
                 if let Some(l) = layout {
                     if l == "horizontal" {
                         flex_dir = FlexDirection::Row;
                     }
                 }
                 
                 let style = Style {
                     flex_direction: flex_dir,
                     gap: Size { width: LengthPercentage::Points(10.0), height: LengthPercentage::Points(10.0) }, // Default gap
                     padding: Rect { 
                         left: LengthPercentage::Points(20.0).into(), 
                         right: LengthPercentage::Points(20.0).into(), 
                         top: LengthPercentage::Points(20.0).into(), 
                         bottom: LengthPercentage::Points(20.0).into() 
                     },
                     size: Size { width: Dimension::Auto, height: Dimension::Auto },
                     ..Default::default()
                 };
                 
                 let node = self.taffy.new_leaf(style).unwrap();
                 
                 for child in children {
                     let child_node = self.build_tree(child);
                     self.taffy.add_child(node, child_node).unwrap();
                 }
                 
                 node
             }
             Value::Image(_) => {
                  let style = Style {
                      size: Size { width: Dimension::Points(100.0), height: Dimension::Points(100.0) }, // Fixed size for now
                      ..Default::default()
                  };
                  self.taffy.new_leaf(style).unwrap()
             }
             Value::String(_) | Value::Number(_) => {
                  // Text node approximation
                  let style = Style {
                      size: Size { width: Dimension::Auto, height: Dimension::Points(24.0) },
                      margin: Rect {
                          left: LengthPercentage::Points(5.0).into(),
                          right: LengthPercentage::Points(5.0).into(),
                          top: LengthPercentage::Points(5.0).into(),
                          bottom: LengthPercentage::Points(5.0).into(),
                      },
                      ..Default::default()
                  };
                  self.taffy.new_leaf(style).unwrap()
             }
             _ => {
                 self.taffy.new_leaf(Style::default()).unwrap()
             }
        }
    }
    
    fn collect_results(&self, node: Node, value: &Value, parent_x: f32, parent_y: f32, results: &mut Vec<(f32, f32, f32, f32, Value)>) {
        let layout = self.taffy.layout(node).unwrap();
        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        results.push((x, y, w, h, value.clone()));
        
        if let Value::Component { children, .. } = value {
             if let Ok(children_nodes) = self.taffy.children(node) {
                 for (i, child_node) in children_nodes.iter().enumerate() {
                     if i < children.len() {
                         self.collect_results(*child_node, &children[i], x, y, results);
                     }
                 }
             }
        }
    }
}
