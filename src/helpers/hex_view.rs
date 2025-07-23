// Take in a vec of bytes and return a hex view of it

pub fn hex_view(bytes: Vec<u8>) -> String {
    let mut offset = 0;
    let bytes_length = bytes.len();

    let mut screen_string = String::new();

    // push the header
    screen_string.push_str(" Offset(h)  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F\n");

    while offset < bytes_length {
        // make the line
        let mut line = String::new();
        // first goes the offset, padded so its 10 characters long
        line.push_str(&format!("{offset:0>10X}  "));
        // now for all the numbers
        for i in 0..16 {
            // skip if we are outside of range
            if offset + i >= bytes_length {
                line.push_str("  ");
            } else {
                let byte = bytes[offset + i];
                let byte_component = format!("{byte:02X} ");
                line.push_str(&byte_component);
            }
        }

        // now for the text version
        line.push(' ');
        for i in 0..16 {
            let mut character: char;
            if offset + i >= bytes_length {
                character = ' ';
            } else {
                // convert
                let byte = bytes[offset + i];
                character = char::from_u32(byte as u32).unwrap_or('?');
                // unless:
                if !character.is_ascii() || character.is_ascii_control() {
                    character = '.';
                }
            }

            line.push(character);
        }

        // line is done. Add it to the screen
        screen_string.push_str(&line);
        screen_string.push('\n');

        // Now increment the offset
        offset += 16;
    }

    // done!
    screen_string
}
