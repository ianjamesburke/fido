use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout, Write};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal
/// Note: Mouse capture is intentionally NOT enabled for keyboard-only navigation
pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    
    // Explicitly disable mouse tracking with ANSI escape sequences
    // This prevents the terminal from sending mouse events entirely
    print!("\x1b[?1000l"); // Disable X11 mouse reporting
    print!("\x1b[?1002l"); // Disable cell motion mouse tracking
    print!("\x1b[?1003l"); // Disable all motion mouse tracking
    print!("\x1b[?1006l"); // Disable SGR extended mouse mode
    io::stdout().flush()?;
    
    // Windows-specific: Disable mouse input at the console level
    #[cfg(windows)]
    disable_windows_mouse_input()?;
    
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Windows-specific function to disable mouse input at the console API level
#[cfg(windows)]
fn disable_windows_mouse_input() -> Result<()> {
    use windows::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_MOUSE_INPUT,
        STD_INPUT_HANDLE,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE)
            .map_err(|e| anyhow::anyhow!("Failed to get console handle: {}", e))?;
        
        let mut mode = CONSOLE_MODE(0);
        GetConsoleMode(handle, &mut mode)
            .map_err(|e| anyhow::anyhow!("Failed to get console mode: {}", e))?;
        
        // Remove ENABLE_MOUSE_INPUT flag to disable mouse events
        mode &= !ENABLE_MOUSE_INPUT;
        
        SetConsoleMode(handle, mode)
            .map_err(|e| anyhow::anyhow!("Failed to set console mode: {}", e))?;
    }

    Ok(())
}

/// Restore the terminal to its original state
pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    // Windows-specific: Re-enable mouse input to restore original console state
    #[cfg(windows)]
    enable_windows_mouse_input()?;
    
    Ok(())
}

/// Windows-specific function to re-enable mouse input at the console API level
#[cfg(windows)]
fn enable_windows_mouse_input() -> Result<()> {
    use windows::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_MOUSE_INPUT,
        STD_INPUT_HANDLE,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE)
            .map_err(|e| anyhow::anyhow!("Failed to get console handle: {}", e))?;
        
        let mut mode = CONSOLE_MODE(0);
        GetConsoleMode(handle, &mut mode)
            .map_err(|e| anyhow::anyhow!("Failed to get console mode: {}", e))?;
        
        // Add ENABLE_MOUSE_INPUT flag to restore mouse events
        mode |= ENABLE_MOUSE_INPUT;
        
        SetConsoleMode(handle, mode)
            .map_err(|e| anyhow::anyhow!("Failed to set console mode: {}", e))?;
    }

    Ok(())
}
