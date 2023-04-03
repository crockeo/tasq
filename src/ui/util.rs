use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

pub fn handle_input_single_line(target: &mut String, evt: KeyEvent) {
    match evt.code {
        KeyCode::Char(c) => {
            target.push(c);
        }
        KeyCode::Backspace => {
            target.pop();
        }
        _ => {}
    }
}

pub fn handle_input_multi_line(target: &mut String, evt: KeyEvent) {
    match evt.code {
        KeyCode::Char(c) => {
            target.push(c);
        }
        KeyCode::Enter => {
            target.push('\n');
        }
        KeyCode::Backspace => {
            target.pop();
        }
        _ => {}
    }
}

pub fn cursor_offset(target: &str) -> (u16, u16) {
    // TODO: fix some things with this:
    // - handle wrapping
    //   - make the `x` coordinate fit horizontally along the rune-length of the last section of a wrapped line
    //   - make the `y` coordinate account for the vertical length of wrapped lines
    let lines: Vec<&str> = target.split('\n').collect();
    let mut x = 1; // offset by one so the cursor falls on the point _after_ the last character
    if let Some(last_line) = lines.last() {
	let chars: Vec<char> = last_line.chars().collect();
	x += chars.len() as u16;
    }
    let y = lines.len() as u16 - 1;
    (x, y)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_output__empty() {
	let offset = cursor_offset("");
	assert_eq!(offset, (1, 0));
    }

    #[test]
    fn test_cursor_output__single_line() {
	let offset = cursor_offset("here's some characters");
	assert_eq!(offset, (23, 0));
    }

    #[test]
    fn test_cursor_output__single_line_rune() {
	// note the special `’` is multiple bytes long
	let offset = cursor_offset("here’s some characters");
	assert_eq!(offset, (23, 0));
    }

    #[test]
    fn test_cursor_output__multi_line() {
	// note the special `’` is multiple bytes long
	let offset = cursor_offset("here's some characters\nacross multiple lines");
	assert_eq!(offset, (22, 1));
    }
}
