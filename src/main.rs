
use clap::Parser; // cargo add clap --features derive
use toml::Table; // cargo add ‎toml
use std::fs;
use toml::Value;
use chrono; // cargo add chrono
use chrono::Datelike;

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
	// let year = date.year();
	// let month = date.month();
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
// 
// 	println!();
// 	println!("income: {income}");
// 	println!("expenditures: {expenditures:?}");

	// println!();
	// for (idx, money) in expenditures.iter().enumerate() {
	// 	let day = idx + 1;
	// 	println!("{day}: {money}");
	// }

	// println!();

	let money_per_day = income / days_in_month as f64; // whatevert just use a cast // TODO see if we can do it the other way

	let mut ballance = vec![(0.0_f64, 0.0_f64); days_in_month];

	let mut money_so_far = 0.0_f64;

	for idx in 0..ballance.len(){
		let bal_this_day = money_per_day - expenditures[idx];
		money_so_far += bal_this_day;
		ballance[idx] = (money_so_far, bal_this_day);
	}

	let ballance = ballance;

	println!("day_of_month: ballance_so_far [ballance_this_day]");

	for (idx, (ballance_so_far, ballance_this_day)) in ballance.iter().enumerate() {
		let day = idx + 1;

		print!("{day:2}: {ballance_so_far:7.2} [{ballance_this_day:6.2}]");
		if day == today.try_into().unwrap() {
			println!(" <");
		}else{
			println!();
		}
	}
}
