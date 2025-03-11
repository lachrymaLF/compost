use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;
use chrono::Datelike;
use glob::glob;
use regex::Regex;

/*
fn typer_tags() {}

fn process_c() {}
 */

fn fill_template(template_file: &Path, map: HashMap<&str, String>) -> String {
	let mut contents = read_to_string(&template_file).unwrap();

    for (key, value) in map.into_iter() {
        contents = contents.replace(key, value.as_str());
    }

    return contents;
}

fn compose(filename: &Path, _lib_dir: &Path, _include_dir: &Path, _c_dir: &Path, template: &Path, output_dir: &Path, copy_year: i32) {
	let mut contents: String = read_to_string(&filename).unwrap();

	let mut lines = contents.lines();
	let title = lines.next().unwrap()[2..].to_owned();
	let description = lines.next().unwrap()[2..].to_owned();
	let thumb = match lines.next().unwrap() {
        line if Regex::new(r"%\s").unwrap().is_match(line) => line[2..].to_owned(),
        _ => "https://lachrymal.net/thumbnails/default.png".to_string(),
    };

	println!("Composing {} (\"{}\")...", filename.display(), title);

	contents = Regex::new(r"(?m)^%\s*.*").unwrap().replace_all(&contents, "").to_string();

    let inject_head_re = Regex::new(r"(?s)(<\#inject_head\#>(.*)</\#inject_head\#>)").unwrap();
	
	let head_injection = if let Some(head) = inject_head_re.captures(&contents) {
        head.get(2).unwrap().as_str().to_string()
	}
    else {
        "".to_string()
    };

    if head_injection != "" {
        contents = inject_head_re.replace(&contents, "").to_string();
    }
	
	// contents = typer_tags(&contents);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&contents));
	
	let map: HashMap<&str, String> = HashMap::from([
    	("`META_PAGE_TITLE`", html_escape::encode_safe(&title).to_string()),
        ("`PAGE_TITLE`", title),
		("`META_PAGE_DESCRIPTION`", html_escape::encode_safe(&description).to_string()),
		("`PAGE_DESCRIPTION`", description),
		("`CONTENT`", html),
		("`YEAR`", copy_year.to_string()),
		("`META_THUMBNAIL`", thumb),
		("`HEAD_INJECT`", head_injection)
    ]);
	
	let output_filename = output_dir.join(Regex::new(r"(?i)(.*/)?([A-Za-z0-9_-]+)\.md").unwrap().captures(filename.to_str().unwrap()).unwrap().get(2).unwrap().as_str().to_owned() + ".html");
	
	let out = fill_template(&template, map);

	// out = process_c(&out, lib_dir, include_dir, c_dir);

    fs::write(output_filename, out).unwrap();
}

fn main() {
    let lib_dir         = Path::new("./lib/");
    let include_dir     = Path::new("./include/");
    let c_dir           = Path::new("./bin/");
    let template_dir    = Path::new("./templates/");
    let content_dir     = Path::new("./content/");
    let output_dir      = Path::new("./out/");
    let root_dir        = Path::new("/var/www/lachrymal.net/public_html");
    let docroot_dir   = root_dir.join("/indev/");
    let sync_to_docroot  = false;
    let copy_year         = chrono::Utc::now().year();

    println!("   ______                                 __ 
  / ____/___  ____ ___  ____  ____  _____/ /_
 / /   / __ \\/ __ `__ \\/ __ \\/ __ \\/ ___/ __/
/ /___/ /_/ / / / / / / /_/ / /_/ (__  ) /_  
\\____/\\____/_/ /_/ /_/ .___/\\____/____/\\__/  
                    /_/                      ");

    println!("Processing...");
    println!("=============");

    fs::remove_dir_all(c_dir).unwrap();
    fs::create_dir(c_dir).unwrap();
    fs::remove_dir_all(output_dir).unwrap();
    fs::create_dir(output_dir).unwrap();

    let pages = glob(content_dir.join("*").to_str().unwrap()).unwrap();
    for page in pages {
    	compose(&page.unwrap(), lib_dir, include_dir, c_dir, &template_dir.join("lach.html"), output_dir, copy_year);
    }

    if sync_to_docroot {
        println!("Updating website...");
        println!("===================");
        for html in glob(docroot_dir.join("*.html").to_str().unwrap()).unwrap() {
            match html {
                Ok(path) => fs::remove_file(path).unwrap(),
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Command::new("rsync")
                .arg("-avh")
                .arg(output_dir)
                .arg(docroot_dir)
                .spawn()
                .unwrap();
    }
}
