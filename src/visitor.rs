use crate::parser::Muster;

pub struct Visitor<'a> {
    muster: &'a Muster,
}
impl<'a> Visitor<'a> {
    pub(crate) fn new(muster: &'a Muster) -> Self {
        Self { muster }
    }

    pub(crate) fn visit_muster(&self) {
        for logs_block in self.muster.logs_blocks.iter() {
            println!("Application name: {}", logs_block.application_name);
            println!("Duration: {:?}", logs_block.duration);
            println!("Frequency: {}", logs_block.frequency);
            println!("Templates: {}", logs_block.templates.len());
            println!("Flows: {:?}", logs_block.flows);
            println!("Data: {:?}", logs_block.data);
            println!("Context: {:?}", logs_block.context);
        }
    }
}
