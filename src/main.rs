
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

	let data = fs::read_to_string(args.bills_toml)
		.unwrap();

	let data = data.parse::<Table>()
		.unwrap();

	let mut income: f64 = 0.0;
	let mut expenditures_monthly: f64 = 0.0;
	let mut expenditures = vec![0.0; 31];

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
							expenditures[day] += money; // panic here if out of bound
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

	println!();
	println!("income: {income}");
	println!("expenditures: {expenditures:?}");

	println!();
	for (idx, money) in expenditures.iter().enumerate() {
		let day = idx + 1;
		println!("{day}: {money}");
	}

	let today = chrono::offset::Local::now().date().year();

	println!();
	dbg!(today);
}
