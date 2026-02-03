use super::App;

impl App {
    /// Switch to next tab
    pub fn next_tab(&mut self) {
        let next = self.current_tab.next();
        self.try_switch_tab(next);
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        let prev = self.current_tab.previous();
        self.try_switch_tab(prev);
    }
}
