// how da tui looks.

use std::{time::{Duration, Instant}};

use log::error;
use ratatui::{crossterm, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style, Stylize}, text::Text, widgets::{Block, Borders, Clear, Gauge, List}, Frame};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};
use tui_textarea::{Input, Key};

use crate::tui::{prompts::TuiPrompt, state::FlusterTUIState, tasks::TaskInfo};

/// The TUI interface of fluster. Call methods on this to update the interface as often as you'd like.
pub struct FlusterTUI<'a> {
    /// The actual internal state
    pub(super) state: FlusterTUIState,
    /// The last time the interface was updated
    pub(super) last_update: Instant,
    /// When fluster was first launched.
    pub(super) started: Instant,
    /// User prompt, if any.
    pub(super) user_prompt: Option<TuiPrompt<'a>>,
}


// Drawing time!
impl FlusterTUI<'_> {
    /// Draw the TUI interface for Fluster
    /// 
    /// Takes in a frame from the terminal.
    pub fn draw(&mut self, frame: &mut Frame) {
        // Split the window into sections
        let layout = Layout::default().margin(0).direction(Direction::Vertical).constraints([
            // Progress bars
            Constraint::Ratio(3, 6),
            // Statistics
            Constraint::Ratio(2, 6),
            // Logging
            Constraint::Ratio(1, 6),
        ]).split(frame.area());

        // Draw the progress bars if needed.
        let progress_area = layout[0];
        if let Some(task) = &self.state.task {
            // Collect information about the sub tasks if there are any
            let tasks = task.get_tasks_info();

            // Now we need to put all of the progress bars we'll be creating into a
            // block, then we will put the block into the layout at the end.
            let container_block: Block = Block::bordered().title("Running tasks:");

            // Now based on the number of tasks we have, and how much room we get
            // from layout constraints, we will draw the progress bars.

            // If we have more bars than can be drawn into the space we have, we will
            // truncate the list from the front, effectively scrolling it upwards.

            // Gauges are 3 characters tall, top border, bar, and bottom border.
            
            // Get the amount of vertical space we have inside of our containing block
            let inner_height = &container_block.inner(progress_area).height;
            // Now, how many bars will fit?
            let max_tasks = inner_height / 3;

            // if we cant render any tasks, the window is too small.
            if max_tasks == 0 {
                return // just skip rendering entirely, lazy solution.
            }

            // Since we wouldn't be able to render them anyways, we can discard any tasks
            // we dont need.
            // Skip as many as we cant render. Saturating subtraction, because we can
            // have more room than tasks.
            let incoming_tasks = tasks.len();
            let visible_tasks: Vec<TaskInfo> = tasks.into_iter().skip(incoming_tasks.saturating_sub(max_tasks.into())).collect();

            // Render the container block, not sure why this needs to happen now, but im writing practically the entire
            // tui before testing it so... YOLO
            frame.render_widget(&container_block, progress_area);

            
            // Now we need constraints for each of the gauges to render into.
            let bar_layout = Layout::vertical(
                vec![Constraint::Length(3); visible_tasks.len()]
            ).split(container_block.inner(progress_area));

            // Now for each of the task progress bars that are visible, we render
            // them into their constraints.
            // Man, UI logic is confusing.
            // Zipping so i can iterate in pairs

            // Also for fun factor, each bar will have a different color.
            // We'll cycle through them.
            // Would do rainbow, but orange is missing!

            let colors: Vec<ratatui::style::Color> = vec![
                Color::LightRed,
                Color::LightYellow,
                Color::LightCyan,
                Color::LightGreen,
                Color::LightMagenta
            ];
            let mut color_index = 0;


            for (task, area) in visible_tasks.iter().zip(bar_layout.iter()) {
                // Make the gauge
                let gauge = Gauge::default()
                    // Surround the gauge with a border, and put the title of the task on it
                    .block(
                        Block::default()
                        .borders(
                            Borders::ALL
                        )
                        .title(task.name())
                        // Add a note for how long this has been running
                        .title_bottom(format!("{} | Guesstimated time remaining: {}",task.time_passed(), task.time_remaining()))
                    )
                    // Cycling colors
                    .gauge_style(Style::default().fg(colors[color_index]))
                    // Add the percentage
                    .ratio(task.progress());

                // Render the gauge into its area
                frame.render_widget(gauge, *area);

                // Increment color, looping around
                color_index = (color_index + 1) % colors.len();
            }
        } else {
            // No tasks to display.
            let container_block: Block = Block::bordered().title("Idle, waiting for tasks...");
            frame.render_widget(&container_block, progress_area);
        }
        // Done with progress bars

        // Now for the statistics.
        let statistics_area = layout[1];

        // Fancy box for the statistics
        let stat_box: Block = Block::bordered().title("Statistics:");

        // We will split the statistics window in two horizontally.
        // One side is for disk related information, the other is for cache.

        // Get the size of the stat box
        let stat_box_size = &stat_box.inner(statistics_area);

        // Split it in half
        let stat_layout = Layout::default().margin(0).direction(Direction::Horizontal).constraints([
            // Left
            Constraint::Ratio(1, 2),
            // Right
            Constraint::Ratio(1, 2),
        ]).split(*stat_box_size);
        let stat_left: Rect = stat_layout[0];
        let stat_right: Rect = stat_layout[1];

        // Render the box
        frame.render_widget(&stat_box, statistics_area);

        //
        // Disk statistics
        //

        let disk_box: Block = Block::bordered().title("Disk:");
        
        // We'll use a list for statistics.

        // Turn all the info into strings.
        let mut disk_strings: Vec<String> = Vec::with_capacity(3);
        disk_strings.push(format!("Disks swapped: {}", self.state.disk_swap_count));
        disk_strings.push(format!("Blocks read: {}", self.state.disk_blocks_read));
        disk_strings.push(format!("Blocks written: {}", self.state.disk_blocks_written));
        // Now a newline for spacing
        disk_strings.push("".to_string());
        // And the current disk in the drive
        disk_strings.push(format!("Current disk in drive: {}", self.state.current_disk_in_drive));
        disk_strings.push("".to_string());
        // How long fluster has been running
        let seconds =  self.started.elapsed().as_secs();
        let minutes = seconds / 60;
        let hours = minutes / 60;
        disk_strings.push(format!("Uptime: [{:0>2}:{:0>2}:{:0>2}]", hours, minutes%60, seconds%60));

        // Listify it.
        // We'll also surround it with our block
        let disk_list: List = List::new(disk_strings).block(disk_box);

        // Render that into the left side
        frame.render_widget(disk_list, stat_left);

        //
        // Disk statistics
        //

        let cache_box: Block = Block::bordered().title("Cache:");
        let cache_box_size = cache_box.inner(stat_right);
        frame.render_widget(&cache_box, stat_right);

        // The cache is a bit more complicated, since we need to throw a gauge in there to
        // visualize the current cache hit rate.

        // So we split our space again.
        let cache_split = Layout::default().margin(0).direction(Direction::Vertical).constraints([
            // 3 lines for each gauge
            Constraint::Min(3),
            Constraint::Min(3),
            // The rest of the room is for other stats.
            Constraint::Percentage(100),
        ]).split(cache_box_size);
        let hit_gauge_space = cache_split[0];
        let pressure_gauge_space = cache_split[1];
        let cache_text = cache_split[2];

        // Make the hit gauge
        let hit_gauge: Gauge = Gauge::default()
        .block(Block::bordered()
        .title("Cache hit rate:"))
        .ratio(self.state.cache_hit_rate);
        
        // Render it in
        frame.render_widget(hit_gauge, hit_gauge_space);

        // Make the pressure gauge
        let hit_gauge: Gauge = Gauge::default()
        .block(Block::bordered()
        .title("Cache pressure:"))
        .ratio(self.state.cache_pressure);
        frame.render_widget(hit_gauge, pressure_gauge_space);

        // Now for the boring text.

        let mut cache_strings: Vec<String> = Vec::with_capacity(3);

        cache_strings.push(format!("Swaps saved: {}", self.state.cache_swaps_saved));
        cache_strings.push(format!("Reads cached: {}", self.state.cache_blocks_read));
        cache_strings.push(format!("Writes cached: {}", self.state.cache_blocks_written));
        cache_strings.push(format!("Cache flushes: {}", self.state.cache_flushes));
        
        let cache_list: List = List::new(cache_strings);

        // Render that into the remaining stats space
        frame.render_widget(cache_list, cache_text);


        // Now finally, for the logs
        let logs_area = layout[2];

        // There's a widget for this!
        let logs = TuiLoggerWidget::default()
            .block(
                Block::bordered().title("Logs:")
            )
            .style_error(Style::default().red())
            .style_warn(Style::default().yellow())
            .style_info(Style::default().white())
            .style_debug(Style::default().green())
            .style_trace(Style::default().light_blue())
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated));

        // Render it in!
        frame.render_widget(logs, logs_area);

        // Finally, handle pop-ups if needed.
        pop_up_handler(frame, &mut self.user_prompt);

        // Update the render time
        self.last_update = Instant::now();

        // Done!
    }
}

// Display pop-ups and prompt for input.
fn pop_up_handler(frame: &mut Frame, incoming_pop_up: &mut Option<TuiPrompt>) {

    // Grab the popup if it exists.
    let pop_up = if let Some(up) = incoming_pop_up {
        up
    } else {
        // No pop up, nothing to do.
        return
    };

    // We'll be putting a box in the middle of the screen.
    // Annoyingly we have to create a grid then just pull the middle out
    let row  = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Ratio(1, 10),
        Constraint::Ratio(8, 10),
        Constraint::Ratio(1, 10),
    ]).split(frame.area())[1];
    let popup_layout = Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Ratio(1, 10),
        Constraint::Ratio(8, 10),
        Constraint::Ratio(1, 10),
    ]).split(row)[1];
    // Darken everything by dimming everything
    let frame_size = frame.area();
    frame.buffer_mut().set_style(frame_size, Style::new().dim());

    // Clear out the area that the pop-up is about to draw into
    frame.render_widget(Clear, popup_layout);

    // Now make the prompting window.
    let pop_up_block = Block::bordered()
        .title(pop_up.title.clone())
        .title_alignment(Alignment::Center)
        // Blinking!
        .border_style(if pop_up.flash {Style::new().slow_blink().red().on_black()} else {Style::new().white().on_black()})
        .border_set(ratatui::symbols::border::FULL)
        // Make the inside of the pop-up white on cyan.
        .style(Style::new().on_cyan().white());

    // Get the side of the inside of that
    let popup_inside = pop_up_block.inner(popup_layout);

    // Render the pop up window
    frame.render_widget(pop_up_block, popup_layout);

    // Now for the inside of the window.
    // Split into 2 parts, the top for the message, and the bottom for the input.

    let top_bottom = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Ratio(7, 10),
        Constraint::Ratio(3, 10),
    ]).split(popup_inside);
    let top = top_bottom[0];
    let bottom = top_bottom[1];

    // Assemble the top half of the window, which is just text.
    let text = Text::from(pop_up.content.clone()).centered();

    // Render it
    frame.render_widget(text, top);


    // And finally, for the part the user interacts with.
    // If the caller wants a string back, we'll return it. Otherwise we'll just discard it.
    
    // Render either the text box, or the continue prompt.
    if pop_up.expects_string {
        frame.render_widget(&pop_up.text_area, bottom);
    } else {
        let prompt = Text::from("Press enter to continue.").centered();
        frame.render_widget(prompt, bottom);
    }

    // Now we check for input, if it's enter, we will return the string if needed, and
    // remove the prompt.

    // But we don't block here, we just see if a key has been pressed.
    // If the polling fails, we just move on, and hope it gets hit next time.
    let event_waiting = crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false);
    if !event_waiting {
        // No events, so we're done.
        return
    }

    // Handle the event.
    let read = if let Ok(event) = crossterm::event::read() {
        event
    } else {
        // Tried to read, got nothing. Hopefully we'll catch this on the next time around.
        return
    };

    match read.into() {
        Input {
            key: Key::Esc | Key::Enter,
            ..
        } => {
            // The user has exited the prompt, we're done!
            // To respond, we need to pull out the TuiPrompt and swap a None into its place.
            // Higher up we already checked that the pop-up exists, so we can safely extract it.
            let extracted = incoming_pop_up.take().expect("Guarded, already checked that the pop-up exists.");

            
            // We always send the string back, caller will toss it if they dont need it.
            // It would be really strange for the caller to have died, since they should just be blocking until hearing a result, so
            // if they dont eat the result, there's not really much we can do.
            let _ = extracted.callback.send(extracted.text_area.into_lines().concat());
        },
        input => {
            // User typed
            // This still updates the invisible text area, even if we
            // dont expect input.
            let _ = &pop_up.text_area.input(input);
        }
    }    
}