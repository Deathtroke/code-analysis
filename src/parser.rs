use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;


pub fn parse(input :&str) -> Pair<Rule>{
    let pair = MyParser::parse(Rule::command, input)
        .expect("unsuccessful parse")
        .next().unwrap();

    pair
}