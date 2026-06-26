use core_graphics::display::CGDisplay;

pub struct DisplaySensor{
    display: CGDisplay
}

impl DisplaySensor{
    pub fn new() -> Self{
        Self{
            display: CGDisplay::main()
        }
    }
    
    pub fn sleeping(&self) -> bool{
        self.display.is_asleep()
    }
}