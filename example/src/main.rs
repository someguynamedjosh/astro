use astro_gui::*;

struct Config;

impl GuiConfig for Config {
    type Renderer = ();
}

fn main() {
    let list = Column::new::<Config>(vec![DebugRect, DebugRect, DebugRect]);
    let mut root = AlignBox::new::<Config>(Center, Middle, list);
    let drawer = astro_gui::GuiDrawer::new();
    drawer.layout::<Config, _>(&mut root);
    let commands = drawer.draw::<Config, _>(&root);
    println!("{:#?}", commands);
}
