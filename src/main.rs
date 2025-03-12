use std::fs::{create_dir, remove_file, remove_dir_all, read_to_string, write};
use std::path::Path;
use std::process::Command;
use chrono::Datelike;
use glob::glob;
use regex::Regex;
use uuid::Uuid;
use rayon::prelude::*;

const CC: &str = "gcc";

fn do_c(html: &mut String, basename: &str, lib_dir: &Path, include_dir: &Path, c_dir: &Path) {
    let c_re = Regex::new(r"(?s)<c>(.*?)</c>").unwrap();
    while let Some(capture) = c_re.captures(&html) {
        let source_match = capture.get(1).unwrap();

        let source = format!(r#"
#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <string.h>
#include <stdarg.h>
#include <math.h>
#include <time.h>
#include <sys/stat.h>
#include <sys/types.h>

// Utility library inline C code can use to generate cool stuff
#include "kpw_web_utils.h"

// Built-in macros
#define Length(arr) \\
arr ## _LEN

#define HashValue(hash, key) \\
hash ## _VALS[ \\
    HashValueIndex(key, hash ## _KEYS, hash ## _LEN) \\
]

// Built-in functions (just a lazy linear search for a string array index rn)
int HashValueIndex(const char *key, const char **keys_arr, size_t keys_arr_len)
{{
    for (int i = 0; i < keys_arr_len; i++)
    {{
        if (strcmp(keys_arr[i], key) == 0)
            return i;
    }}
    
    return -1;
}}

// This article's .md file
#define THIS_FILE "content/{}.md"

// This article's filename without the extension or path
#define THIS_BASENAME "{}"

int main(void)
{{
    {}
    
    return 0;
}}"#,
            basename,
            if basename == "index" { "home" } else { basename },
            source_match.as_str());

        let id = Uuid::new_v4();

        let c_fn = c_dir.join(format!("src_{}.c", id));
        write(&c_fn, source).unwrap();

        let o_fn = c_dir.join(format!("out_{}", id));
        let out = match Command::new(CC).
            arg(c_fn)
            .args(glob(lib_dir.join("*.o").to_str().unwrap()).unwrap().map(|p| p.unwrap()))
            .arg("-lm")
            .arg("-I")
            .arg(include_dir)
            .arg("-o")
            .arg(&o_fn)
            .status() {
            Ok(_) => String::from_utf8(Command::new(o_fn).output().unwrap().stdout).unwrap(),
            Err(e) => e.to_string()
        };

        html.replace_range(capture.get(0).unwrap().range(), &out);
    }
}

fn do_typer_tags(contents: &mut String) {
	let typertags_re = Regex::new(r#"\[(\w+)](?:\s+)?\{([^{}]*)\}"#).unwrap();
	let code_re = Regex::new(r"```.*?```").unwrap();

    let mut cit = code_re.find_iter(contents);
    let mut closest_code_block = cit.next();
    let mut replacements: Vec<(core::ops::Range<usize>, String)> = vec![];
    'captures: for capture in typertags_re.captures_iter(&contents) {
        'advance_code: while closest_code_block.is_some() {
            let r = closest_code_block.unwrap().range();
            let start = capture.get(0).unwrap().start();
            if r.contains(&start) {
                continue 'captures
            }
            else if r.end < start {
                closest_code_block = cit.next();
            }
            else {
                break 'advance_code
            }
        }
        let (full, [tag_name, tag_body]) = capture.extract();

        let rep = match tag_name {
            "note" => {
                let mut html = String::new();
                pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&tag_body));
                format!(r#"<div class="note">{html}</div>"#)
            },
            _ => full.to_owned(),
        };

        replacements.push((capture.get(0).unwrap().range(), rep));
    }

    for (range, content) in replacements.into_iter().rev() {
        contents.replace_range(range, &content);
    }
}

fn compose(filename: &Path,
    lib_dir: &Path,
    include_dir: &Path,
    c_dir: &Path,
    template: &Path,
    output_dir: &Path,
    default_thumb:
    &str,
    copy_year: i32
) {
	let mut contents: String = read_to_string(&filename).unwrap();

	let mut lines = contents.lines();
	let title = lines.next().unwrap()[2..].to_owned();
	let description = lines.next().unwrap()[2..].to_owned();
	let thumb = match lines.next() {
        Some(d) => match d {
            line if Regex::new(r"%\s").unwrap().is_match(line) => line[2..].to_owned(),
            _ => default_thumb.to_string(),
        }
        None => default_thumb.to_string()
    };

	println!("Composing {} (\"{}\")...", filename.display(), title);

	contents = Regex::new(r"(?m)^%\s*.*").unwrap().replace_all(&contents, "").to_string();

	let head_capture = Regex::new(r"(?s)<\#inject_head\#>(.*?)</\#inject_head\#>").unwrap().captures(&contents);
    let (head_injection, head_range) = match head_capture {
        Some(head) => (head.get(1).unwrap().as_str().to_string(), Some(head.get(0).unwrap().range())),
        None => ("".to_string(), None),
	};

    if let Some(r) = head_range {
        contents.replace_range(r, "");
    }

	do_typer_tags(&mut contents);

    let basename: String = Regex::new(r"(?i)(.*/)?([A-Za-z0-9_-]+)\.md").unwrap().captures(filename.to_str().unwrap()).unwrap().get(2).unwrap().as_str().to_owned();
    do_c(&mut contents, &basename, &lib_dir, &include_dir, &c_dir);

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&contents));

	let map = [
    	("`META_PAGE_TITLE`", html_escape::encode_safe(&title).to_string()),
        ("`PAGE_TITLE`", title),
		("`META_PAGE_DESCRIPTION`", html_escape::encode_safe(&description).to_string()),
		("`PAGE_DESCRIPTION`", description),
		("`CONTENT`", html),
		("`YEAR`", copy_year.to_string()),
		("`META_THUMBNAIL`", thumb),
		("`HEAD_INJECT`", head_injection)
    ];

	let output_filename = output_dir.join(basename.clone() + ".html");

    let mut out = read_to_string(&template).unwrap();

    for (key, value) in map {
        out = out.replace(key, value.as_str());
    }

	do_c(&mut out, &basename, &lib_dir, &include_dir, &c_dir);

    write(output_filename, out).unwrap();
}

fn main() -> Result<(), std::io::Error> {
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
    let default_thumb    = "https://lachrymal.net/thumbnails/default.png";

    println!("   ______                                 __ 
  / ____/___  ____ ___  ____  ____  _____/ /_
 / /   / __ \\/ __ `__ \\/ __ \\/ __ \\/ ___/ __/
/ /___/ /_/ / / / / / / /_/ / /_/ (__  ) /_  
\\____/\\____/_/ /_/ /_/ .___/\\____/____/\\__/  
                    /_/                      ");

    println!("Processing...");
    println!("=============");

    remove_dir_all(output_dir)?;
    create_dir(output_dir).unwrap();
    remove_dir_all(c_dir)?;
    create_dir(c_dir).unwrap();

    let pages: Vec<_> = glob(content_dir.join("*").to_str().unwrap()).unwrap().collect();
    pages.into_par_iter().for_each(|page| compose(
        &page.unwrap(),
        lib_dir,
        include_dir,
        c_dir,
        &template_dir.join("template.html"),
        output_dir,
        default_thumb,
        copy_year
    ));

    if sync_to_docroot {
        println!("Updating website...");
        println!("===================");
        for html in glob(docroot_dir.join("*.html").to_str().unwrap()).unwrap() {
            match html {
                Ok(path) => remove_file(path).unwrap(),
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

    Ok(())
}
