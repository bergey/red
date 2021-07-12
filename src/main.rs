use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "hello")]
struct Hello {
    /// Name of the person to greet
    #[structopt(short, long, default_value = "world")]
    name: String,

    /// Number of times to greet
    #[structopt(short, long, default_value = "1")]
    count: u8,
}

fn main() {
    let hello = Hello::from_args();

    for _ in 0..hello.count {
        println!("Hello {}!", hello.name)
    }
}
