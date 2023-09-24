#![feature(map_many_mut)]
use std::{fmt::Display, io::{Write, Read, BufReader, BufRead}, path::Path, process::Stdio};

mod graph;

use clap::Parser;
use graph::Graph;
use itertools::Itertools;
use rand::Rng;

fn open_graphviz<T: Display>(g: &Graph<T>, name: &str) -> std::io::Result<()> {
    let mut dot = std::process::Command::new("dot")
        .arg("-Tpng")
        .arg("-o")
        .arg(name)
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    let dotstr = g.to_dot();
    std::fs::write(format!("{}.dot", name), dotstr.as_bytes())?;
    dot.stdin.as_mut().unwrap().write_all(dotstr.as_bytes())?;
    dot.wait()?;
    std::process::Command::new("xdg-open").arg(name).spawn()?;
    Ok(())
}

fn rand_string(size: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    for _ in 0..size {
        s.push(rng.gen_range(b'A'..b'Z') as char);
    }
    s
}

macro_rules! links {
    ($g:ident {$($from:ident -> $to:ident);*}) => {
        $($g.link($from, $to));*
    };
}

pub fn read_graph(read: impl Read) -> std::io::Result<Graph<String>> {
    let mut g = Graph::new();
    let f = BufReader::new(read);
    for line in f.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let from = words.next().unwrap();
        let to = words.next().unwrap();
        let from = g.new_node_or_get(from.to_owned());
        let to = g.new_node_or_get(to.to_owned());
        g.link(from, to);
    }
    Ok(g)
}

pub fn has_program(name: &str) -> bool {
    std::process::Command::new("which").arg(name).stdin(Stdio::null()).stdout(Stdio::null()).spawn().is_ok()
}

pub fn output_graph_rudimentary<T, W: Write>(g: &Graph<T>, mut writer: W) -> std::io::Result<()>
where T: Display, W: Write {
    let mut cons = g.iter()
        .sorted_by_cached_key(|node| node.id())
        .map(|node| node.outgoing().iter())
        .flatten()
        .map(|arrow| format!("\t{} {}", g[arrow.from].value(), g[arrow.to].value()))
        .collect::<Vec<_>>() ;
    cons.insert(0, "{".into());
    cons.push("}".into());
    let cons = cons.join("\n");
    writer.write_all(cons.as_bytes())
}

#[derive(clap::Parser)]
struct Args {
    /// The file to read the graph from
    input_file: String,

    #[arg(default_value="output.txt")]
    /// The file to write the f_conex to
    output_file: String,

    /// Do not use graphviz to show graphs
    #[arg(long, short, default_value_t=false)]
    no_graphviz: bool,
}

fn main() {
    let args = Args::parse();
    let mut f = std::fs::File::open(&args.input_file).expect("Failed to open file");
    let g = read_graph(&mut f).expect("Failed to read graph");
    let has_dot = has_program("dot");

    let f_conex = g.f_conex();

    if has_dot && !args.no_graphviz {
        open_graphviz(&g, "graph.png").unwrap();
        match std::fs::create_dir("f_conex") {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            _ => panic!("Failed to create directory"),
        }
        f_conex.iter().enumerate().for_each(|(i, g)| open_graphviz(g, &format!("f_conex/graph-fconex-{}.png", i)).unwrap());
    }
    let mut file = std::fs::File::create(&args.output_file).expect("Failed to create file");
    for subgraph in g.f_conex() {
        output_graph_rudimentary(&subgraph, &mut file).expect("Failed to write to file");
        file.write_all(b"\n").expect("Failed to write to file");
    }

    if !has_dot {
        println!("Dica: instale o graphviz para ver os grafos");
    }
}
