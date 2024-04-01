extern crate haiku;

use haiku::app::ROSTER;

const NAME_FIELD_WIDTH: usize = 34;

fn truncate_string(name: &str, mut length: usize) -> String {
	if name.len() <= length {
		return String::from(name);
	}
	if length < 6 {
		length = 6;
	}

	let begin = length / 3 - 1;
	let end = name.len() - (length - 3 - begin);

	let mut truncated_name = String::with_capacity(length);
	truncated_name.push_str(&name[..begin]);
	truncated_name.push_str("...");
	truncated_name.push_str(&name[end..]);
	truncated_name
}

fn main() {
	println!(
		"  team {:>width$} port flags signature",
		"path",
		width = NAME_FIELD_WIDTH
	);
	println!(
		"------ {:->width$}---- ----- ---------",
		" ",
		width = NAME_FIELD_WIDTH + 1
	);

	let team_list = ROSTER
		.get_app_list()
		.expect("Unexpected error getting the list of teams");
	for team in team_list {
		let app_info = match ROSTER.get_running_app_info(&team) {
			Some(info) => info,
			None => continue,
		};
		let path = truncate_string(&app_info.path, NAME_FIELD_WIDTH);
		println!(
			"{:>6} {:>34}{:>5} {:>5} ({})",
			app_info.team, path, app_info.port, app_info.flags, app_info.signature
		);
	}
}
