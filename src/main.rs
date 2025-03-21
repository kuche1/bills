
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
use textplots::{LineStyle, AxisBuilder};
use term_size; // cargo add term_size

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Toml file containing bills data
    #[arg(short, long)]
    bills_toml: String,
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

fn recursively_sum(value: Value) -> f64 {
	match value{
		Value::String(_) | Value::Boolean(_) | Value::Datetime(_) =>
			panic!("unsupported value: {value}"),

		Value::Integer(v) =>
			return v as f64,

		Value::Float(v) =>
			return v,

		Value::Array(v) => {
			let mut sum: f64 = 0.0;
			for value in v{
				sum += recursively_sum(value);
			}
			return sum;
		},

		Value::Table(v) => {
			let mut sum: f64 = 0.0;
			for (_key, value) in v{
				sum += recursively_sum(value);
			}
			return sum;
		},
	}
}

fn main(){
	let args = Args::parse();

	let date = chrono::offset::Local::now().date_naive();
	let days_in_month: usize = date.days_in_month().try_into().unwrap();
	let year = date.year();
	let month = date.month();
	let today = date.day();

	let data = fs::read_to_string(args.bills_toml)
		.unwrap();

	let data = data.parse::<Table>()
		.unwrap();

	let mut income: f64 = 0.0;
	let mut expenditures_monthly: f64 = 0.0;
	let mut expenditures = vec![0.0_f64; days_in_month];

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

	let income = income - expenditures_monthly;
	let expenditures = expenditures;

	let money_per_day = income / days_in_month as f64; // whatevert just use a cast // TODO see if we can do it the other way

	let mut ballance = vec![(0.0_f64, 0.0_f64); days_in_month];

	let mut money_so_far = 0.0_f64;

	for idx in 0..ballance.len(){
		let bal_this_day = money_per_day - expenditures[idx];
		money_so_far += bal_this_day;
		ballance[idx] = (money_so_far, bal_this_day);
	}

	let ballance = ballance;

	///// print

	println!("date: ballance_so_far [ballance_this_day]");

	for (idx, (ballance_so_far, ballance_this_day)) in ballance.iter().enumerate() {
		let day = idx + 1;

		print!("{year:02}-{month:02}-{day:02}: {ballance_so_far:7.2} [{ballance_this_day:6.2}]");
		if day == today.try_into().unwrap() {
			println!(" <");
		}else{
			println!();
		}
	}

	///// graph

	// println!();

	let mut graph_till_today: Vec<(f32, f32)> = vec![(0.0, 0.0)];
    let mut graph_after_today_no_spend: Vec<(f32, f32)> = vec![];
    let mut graph_after_today_avg_spend: Vec<(f32, f32)> = vec![];
    let mut graph_after_today_no_income: Vec<(f32, f32)> = vec![];
    let mut graph_after_today_avg_median: Vec<(f32, f32)> = vec![];

    let avg_spendings = {
    	let mut total = 0.0_f64;
    	for idx in 0..today { // including today
    		total += expenditures[idx as usize];
    	}
    	total / today as f64
    };
    // println!("avg_spendings={avg_spendings}");

    let avg_median_spendings = {
    	let mut spendings = expenditures.clone();
    	spendings.sort_by(|a, b| a.partial_cmp(b).unwrap());

		let items = spendings.len() / 4; // we want to keep hald the array
		let start = items;
		let end = spendings.len() - items;

		spendings.drain(end..);
		spendings.drain(..start);

		spendings.iter().sum::<f64>() / spendings.len() as f64
    };
    // println!("avg_median_spendings={avg_median_spendings}");

	for (idx, (ballance_so_far, _ballance_this_day)) in ballance.iter().enumerate() {
        let day_usize: usize = idx + 1;
		let day_f32 = day_usize as f32;

        let ballance = *ballance_so_far as f32;

		if day_usize <= today.try_into().unwrap() {
			graph_till_today.push((day_f32, ballance));

			if day_usize == today.try_into().unwrap() {
				graph_after_today_no_spend.push((day_f32, ballance));
				graph_after_today_avg_spend.push((day_f32, ballance));
				graph_after_today_no_income.push((day_f32, ballance));
				graph_after_today_avg_median.push((day_f32, ballance));
			}

		}else{
            graph_after_today_no_spend.push((day_f32, ballance));

			let num_day_after_today: f32 = day_f32 - today as f32;

            graph_after_today_avg_spend.push((day_f32, ballance - num_day_after_today * avg_spendings as f32));

            graph_after_today_no_income.push((day_f32, graph_after_today_no_income[0].1));

			graph_after_today_avg_median.push((day_f32, ballance - num_day_after_today * avg_median_spendings as f32))
        }
	}

	println!();

	let (term_width, term_height) = term_size::dimensions().unwrap();

	let graph_width: u32 = term_width.try_into().unwrap();
	let graph_height: u32 = term_height.try_into().unwrap();

	let graph_width = graph_width * 11 / 6; // 300 // 450
	let graph_height = graph_height * 10 / 3; // 180

	println!("green:no-spend purple:avg-median blue:avg red:no-change");
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

		// boundaries style, this actually sucks because those 2 fncs only cover 2/4 boundaries
		// .x_axis_style(LineStyle::Solid)
		// .y_axis_style(LineStyle::Solid)

		// draw

        .lineplot(&Shape::Bars(&graph_till_today)) // Lines Steps Bars

        .linecolorplot(
			&Shape::Lines(&graph_after_today_no_spend),
			RGB8 {
				r: 40,
				g: 200,
				b: 40,
			},
        )

        .linecolorplot(
			&Shape::Lines(&graph_after_today_no_income),
			RGB8 {
				r: 200,
				g: 60,
				b: 60,
			},
        )

        .linecolorplot(
			&Shape::Lines(&graph_after_today_avg_spend),
			RGB8 {
				r: 20,
				g: 20,
				b: 200,
			},
        )

        .linecolorplot(
			&Shape::Lines(&graph_after_today_avg_median),
			RGB8 {
				r: 200,
				g: 100,
				b: 200,
			},
        )

        .nice();
        // .display();
}
