
// TODO this fucking shit sucks, fuck `textplots` you cannot label horizontally, you cannot use real RGB, the graph size parameter is fucking random, you cannot plot a single point
// also: the graph indicators (the Y axis) suck because they reflect the value on the very top pixel, and it looks wrong (one would expect they indicate the pixel in the middle)

// TODO make it able to look at the previouis month(s) and graph them too

use clap::Parser; // cargo add clap --features derive
use toml::Table; // cargo add toml
use std::fs;
use toml::Value;
use chrono; // cargo add chrono
use chrono::Datelike;
use textplots::{Chart, Plot, Shape}; // cargo add textplots
use textplots::ColorPlot;
use rgb::RGB8; // cargo add rgb
use textplots::{LabelFormat, LabelBuilder};
use textplots::{TickDisplay, TickDisplayBuilder};
// use textplots::{LineStyle, AxisBuilder};
use term_size; // cargo add term_size
use std::path::Path;
use clap_lex::OsStrExt; // cargo add clap_lex // this is weird - the compiler told be to `use` this, and yet I had to `cargo add` it

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Toml file containing bills data
    #[arg(short, long)]
    bills_toml: String,
}

#[derive(Clone)]
struct BallancePoint {
	money_before_today: f32,
	money_so_far: f32,
	ballance_today_from_month_avg: f32,
	ballance_today_from_money_so_far: f32,
}

///////////// vvvvv stupid fucking shit (this should have been included in the library) // https://github.com/chronotope/chrono/issues/69
trait NaiveDateExt {
    fn days_in_month(&self) -> i32;
    fn is_leap_year(&self) -> bool;
}

impl NaiveDateExt for chrono::NaiveDate {
    fn days_in_month(&self) -> i32 {
        let month = self.month();
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if self.is_leap_year() { 29 } else { 28 },
            _ => panic!("Invalid month: {}" , month),
        }
    }

    fn is_leap_year(&self) -> bool {
        let year = self.year();
        return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
}
///////////// ^^^^^

fn recursively_sum(value: Value) -> f32 {
	match value{
		Value::String(_) | Value::Boolean(_) | Value::Datetime(_) =>
			panic!("unsupported value: {value}"),

		Value::Integer(v) =>
			return v as f32,

		Value::Float(v) =>
			return v as f32,

		Value::Array(v) => {
			let mut sum: f32 = 0.0;
			for value in v{
				sum += recursively_sum(value);
			}
			return sum;
		},

		Value::Table(v) => {
			let mut sum: f32 = 0.0;
			for (_key, value) in v{
				sum += recursively_sum(value);
			}
			return sum;
		},
	}
}

//////
////// exterpolations
//////
// TODO problema s tezi exterpolate-vashti funkcii e che rabotqt vurhu raw data a ne vurno delti, eto primer: za da napravq hubav exterpolator za ostavashtite mi pari za vseki den na men mi trqbva informaciq za tova kolko sa samite pari, NO ako vmesto tova kato vhod poluchavam nqkakvi delti i vrushtam delti tova izobshto ne e problem

fn exterpolated_data_to_graph_data(data: Vec<f32>, today: f32) -> Vec<(f32, f32)> {
	data
		.iter()
		.enumerate()
		.map(
			|(idx, val)|
				(today + idx as f32, *val)
		)
		.collect()
}

// calculated average change
fn exterpolate_avg(data: &Vec<f32>, new_entries: usize) -> Vec<f32> {
	let mut deltas: Vec<f32> = vec![];

	for idx in 1 .. data.len() {
		let last = data[idx - 1];
		let cur = data[idx];
		deltas.push(cur - last);
	}

	let mut sum: f32 = 0.0;
	for item in &deltas {
		sum += item;
	}
	let avg = sum / deltas.len() as f32;
	// TODO use the sum fnc

	let mut exterpolated: Vec<f32> = vec![*data.last().unwrap()];
	for _ in 0..new_entries {
		let last = *exterpolated.last().unwrap();
		exterpolated.push(last + avg);
	}

	return exterpolated;
}

// removes most extreme deltas then calculates average
fn exterpolate_median_avg(data: &Vec<f32>, new_entries: usize) -> Vec<f32> {
	let mut deltas: Vec<f32> = vec![];

	for idx in 1 .. data.len() {
		let last = data[idx - 1];
		let cur = data[idx];
		deltas.push(cur - last);
	}

	deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());

	let items = deltas.len() / 4; // we'll remove 1/4 of the beginning of the array, and 1/4 of the end
	let start = items;
	let end = deltas.len() - items;

	deltas.drain(end..);
	deltas.drain(..start);
	
	let median_avg_delta = deltas.iter().sum::<f32>() / deltas.len() as f32;

	let mut exterpolated: Vec<f32> = vec![*data.last().unwrap()];
	for _ in 0..new_entries {
		let last = *exterpolated.last().unwrap();
		exterpolated.push(last + median_avg_delta);
	}

	return exterpolated;
}

fn exterpolate_no_spend(ballance: &Vec<BallancePoint>, today: usize) -> Vec<f32> {
	ballance[today - 1 ..]
	.iter()
	.map(|bal| bal.money_so_far)
	.collect()
}

//////
////// main
//////

fn main(){
	// parse cmdline

	let input_toml = {
		let args = Args::parse();

		args.bills_toml
	};

	// get date

	let (days_in_month, year, month, today) = {

		// // this whole thing seems jabroni (not because of the idea but because of rust)

		let date = Path::new(&input_toml);
		//let date = date.file_name().unwrap(); // "2025.03.toml"
		let date = date.file_stem().unwrap(); // "2025.03"
		// dbg!(date);
		let year_month: Vec<_> = date.split(".").collect();
		// dbg!(&year_month);
		let [year, month] = year_month[..] else { panic!() };
		// dbg!(year);
		// dbg!(month);
		let input_year = year.to_str().unwrap().parse::<i32>().unwrap();
		// dbg!(year);
		let input_month = month.to_str().unwrap().parse::<u32>().unwrap();
		// dbg!(month);
		let date = chrono::NaiveDate::from_ymd_opt(input_year, input_month, 1).unwrap();
		let input_days_in_month = date.days_in_month();

		// what data is today?
		let date = chrono::offset::Local::now().date_naive();
		let today_days_in_month: usize = date.days_in_month().try_into().unwrap();
		let today_year = date.year();
		let today_month = date.month();
		let today_day = date.day();

		if (input_year == today_year) && (input_month == today_month) {
			(today_days_in_month, today_year, today_month, today_day)
		}else{
			(input_days_in_month.try_into().unwrap(), input_year, input_month, input_days_in_month.try_into().unwrap())
		}
	};

	let today_usize: usize = today.try_into().unwrap();

	// read file

	let data = fs::read_to_string(input_toml)
		.unwrap();

	let data = data.parse::<Table>()
		.unwrap();

	// calculate income/expenditures

	let (income, expenditures) = {

		let mut income: f32 = 0.0;
		let mut expenditures_monthly: f32 = 0.0;
		let mut expenditures = vec![0.0_f32; days_in_month];

		for item in data{
			let (key, value) = item;
			// println!("{key} = {value}");

			match key.as_str(){
				"INCOME" =>
					income += recursively_sum(value),

				"EXPENDITURES-MONTHLY" =>
					expenditures_monthly += recursively_sum(value),

				"EXPENDITURES-REGULAR" => {

					match value{
						Value::String(_) | Value::Boolean(_) | Value::Datetime(_) | Value::Integer(_) | Value::Float(_) | Value::Array(_) =>
							panic!("unsupported value: {value}"),

						Value::Table(v) => {
							for (day, money) in v{
								let day: usize = day.parse().expect(&format!("`{}` is not a valid month day", day));
								let day = day - 1;
								let money = recursively_sum(money);
								expenditures[day] += money; // panics if out of bound
							}
						},

						// _ => todo!("exp-reg"),
					}

				},

				_ =>
					panic!("unknown key: {key}"),
			}
		}

		income -= expenditures_monthly;

		(income, expenditures)
	};

	// calculate ballance

	let money_per_day = income / days_in_month as f32;

	let ballance = {

		let mut ballance =
			vec![
				BallancePoint {
					money_before_today: 0.0,
					money_so_far: 0.0,
					ballance_today_from_month_avg: 0.0,
					ballance_today_from_money_so_far: 0.0,
				};
				days_in_month
			];

		let mut money_so_far: f32 = 0.0;

		for idx in 0..ballance.len(){
			let days_left_in_month = (ballance.len() - idx) as f32;
			let money_before_today = money_so_far;

			let money_today_from_month_avg = money_per_day - expenditures[idx];
			money_so_far += money_today_from_month_avg;

			let money_today_from_money_so_far = {
				let money = money_before_today;
				// if money < 0.0 {
				// 	money = 0.0;
				// }
				money_today_from_month_avg + (money / days_left_in_month)
			};

			ballance[idx] =
				BallancePoint {
					money_before_today: money_before_today,
					money_so_far: money_so_far,
					ballance_today_from_month_avg: money_today_from_month_avg,
					ballance_today_from_money_so_far: money_today_from_money_so_far,
				};
		}

		ballance
	};

	///// print

	println!("date: ballance_so_far [ballance_today_from_month_avg] [ballance_today_from_money_so_far]");

	for (idx, bal) in ballance.iter().enumerate() {
		let day = idx + 1;
		let money_so_far = bal.money_so_far;
		let ballance_today_from_month_avg = bal.ballance_today_from_month_avg;
		let ballance_today_from_money_so_far = bal.ballance_today_from_money_so_far;

		print!("{year:02}-{month:02}-{day:02}: {money_so_far:7.2} [{ballance_today_from_month_avg:7.2}] [{ballance_today_from_money_so_far:7.2}]");
		if day == today_usize {
			println!(" <");
		}else{
			println!();
		}
	}

	///// graph

	let mut graph_till_today: Vec<(f32, f32)> = vec![(0.0, 0.0)];
	let mut graph_till_today_dynamic_daily_money: Vec<(f32, f32)> = vec![(0.0, 0.0)];

	let mut graph_after_today_dynamic_daily_money_no_spend = vec![];

	for (idx, bal) in ballance.iter().enumerate() {
        let day_usize: usize = idx + 1;
		let day_f32 = day_usize as f32;

		let money_so_far = bal.money_so_far;

		if day_usize < today_usize {
			graph_till_today.push((day_f32, money_so_far));
			graph_till_today_dynamic_daily_money.push((day_f32, bal.ballance_today_from_money_so_far));
		}else{
			if day_usize == today_usize {
				graph_till_today.push((day_f32, money_so_far));
				graph_till_today_dynamic_daily_money.push((day_f32, bal.ballance_today_from_money_so_far));
			}
			graph_after_today_dynamic_daily_money_no_spend.push((day_f32, bal.ballance_today_from_money_so_far));
		}
	}

	let (
		graph_after_today_no_spend,
		graph_after_today_avg_spend,
		graph_after_today_avg_median,
	) = {
		let data = 
			ballance[.. today_usize]
			.iter()
			.map(|bal| bal.money_so_far)
			.collect();

		let days_left = days_in_month - today_usize;

		(
			exterpolated_data_to_graph_data(
				exterpolate_no_spend(&ballance, today_usize),
				today as f32
			),
			exterpolated_data_to_graph_data(
				exterpolate_avg(&data, days_left),
				today as f32
			),
			exterpolated_data_to_graph_data(
				exterpolate_median_avg(&data, days_left),
				today as f32
			),
		)
	};

	let mark_ballance = {
		let now = *graph_till_today.last().unwrap();
		let end = (days_in_month as f32, graph_till_today.last().unwrap().1);
		vec![now, end]
	};

	let mark_dynamic_daily_money = {
		let now = *graph_till_today_dynamic_daily_money.last().unwrap();
		let end = (days_in_month as f32, graph_till_today_dynamic_daily_money.last().unwrap().1);
		vec![now, end]
	};

	println!();

	let (term_width, term_height) = term_size::dimensions().unwrap();

	let graph_width: u32 = term_width.try_into().unwrap();
	let graph_height: u32 = term_height.try_into().unwrap();

	let graph_width = graph_width * 11 / 6; // I'm happy with this, this seem to be consistent with all zoom levels (except maybe the most extreme zoom-in)
	let graph_height = graph_height * 10 / 3; // 180

	println!("green:no-spend purple:avg-median blue:avg red:dynamic-no-spend"); // red:no-change
	// this fucking sucks
	// I need to find the way to print based on this stupid `rgb`
	// or I need to copy the relative functions from the draw create

    Chart
        ::new(graph_width, graph_height, 0.0 /* start x */, days_in_month as f32 /* end x */)

		// show days as `2` rather than `2.0`
		.x_label_format(LabelFormat::Custom(Box::new(move |val| {
			format!("{val}")
		})))

		// show monay as `12.34` rather than `12.3`
		.y_label_format(LabelFormat::Custom(Box::new(move |val| {
			format!("{val:.2}")
		})))

		// how often the money label appears
		.y_tick_display(TickDisplay::Dense) // None Sparse Dense

		// draw

        .lineplot(&Shape::Bars(&graph_till_today)) // Lines Steps Bars
		.lineplot(&Shape::Lines(&mark_ballance))

		.lineplot(&Shape::Lines(&graph_till_today_dynamic_daily_money))
		.lineplot(&Shape::Lines(&mark_dynamic_daily_money))

		// red

		.linecolorplot(
			&Shape::Lines(&graph_after_today_dynamic_daily_money_no_spend),
			RGB8 {
				r: 200,
				g: 40,
				b: 40,
			},
        )

		// green

        .linecolorplot(
			&Shape::Lines(&graph_after_today_no_spend),
			RGB8 {
				r: 40,
				g: 200,
				b: 40,
			},
        )

		// purple

        .linecolorplot(
			&Shape::Lines(&graph_after_today_avg_median),
			RGB8 {
				r: 200,
				g: 100,
				b: 200,
			},
        )

		// blue

        .linecolorplot(
			&Shape::Lines(&graph_after_today_avg_spend),
			RGB8 {
				r: 20,
				g: 20,
				b: 200,
			},
        )

		// flush

        .nice();
        // .display();
}
