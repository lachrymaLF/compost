use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;
use chrono::Datelike;
use glob::glob;
use regex::Regex;
use uuid::Uuid;

fn do_c(html: &mut String, basename: &str, lib_dir: &Path, include_dir: &Path, c_dir: &Path) {
    fs::remove_dir_all(c_dir).unwrap();
    fs::create_dir(c_dir).unwrap();

    while let Some(capture) = Regex::new(r"(?s)(<c>(.*?)</c>)").unwrap().captures(&html) {
        let source_match = capture.get(2).unwrap();

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
#define THIS_FILE "{}.md"

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
        fs::write(&c_fn, source).unwrap();
        
        let o_fn = c_dir.join(format!("out_{}", id));
        let out = match Command::new("gcc").
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

        html.replace_range(capture.get(1).unwrap().range(), &out);
    }
}

fn do_typer_tags(contents: &mut String) {
	let typertags_re = Regex::new(r#"/\[(\w+)([\s\w=\"-_~]+)?\]([\s]+)?\{([^{}]*)\}/"#).unwrap();
	let code_re = Regex::new(r"/(```([\w-]+))\n([^`]*)(```\n)/").unwrap();
	
	// Figure out how many code blocks there are, so we don't process typertags inside them
	// my @code_blocks = ();
	// while ($contents =~ /($code_regex)/igs) {
	// 	push @code_blocks, { language => $3, text => $4 };
	// }
	
	// Now do the fun stuff; replace all "typertags" (e.g. [b]{asdf} -> <b>asdf</b>)
	// while ($contents =~ /($typertags_regex)/igs) {
	// 	my $tag_name = $2;
	// 	my $tag_attributes = $3;
	// 	my $tag_body = $5;

	// 	if ($tag_name eq "note") {
	// 		$tag_name = "div";
	// 		$tag_attributes = "class=\"side-note\"";
	// 		$tag_body = TyperTag($tag_body, 1);
	// 	}
		
	// 	$contents =~ s/$typertags_regex/<$tag_name $tag_attributes>$tag_body<\/$tag_name>/igs;
	// }
	
	// Revert any changes typertags might've made to code blocks
	// my $i = 0;
	// while ($contents =~ /($code_regex)/igs) {
	// 	my @cb = %{$code_blocks[$i]};
	// 	my $language = $code_blocks[$i]{'language'};
	// 	my $text = ($code_blocks[$i]{'text'});
		
	// 	# Apply the changes
	// 	my $first = quotemeta($1);
	// 	$text = EscapeHTML($text);
	// 	$contents =~ s/$first/<pre><code class="$language">$text<\/code><\/pre>/igs;
	// 	$i++;
	// }
}

fn compose(filename: &Path, lib_dir: &Path, include_dir: &Path, c_dir: &Path, template: &Path, output_dir: &Path, copy_year: i32) {
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

	let head_capture = Regex::new(r"(?s)(<\#inject_head\#>(.*?)</\#inject_head\#>)").unwrap().captures(&contents);
    let (head_injection, head_range) = match head_capture {
        Some(head) => (head.get(2).unwrap().as_str().to_string(), Some(head.get(1).unwrap().range())),
        None => ("".to_string(), None),
	};

    if let Some(r) = head_range {
        contents.replace_range(r, "");
    }
	
	do_typer_tags(&mut contents);
    
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
	
    let basename: String = Regex::new(r"(?i)(.*/)?([A-Za-z0-9_-]+)\.md").unwrap().captures(filename.to_str().unwrap()).unwrap().get(2).unwrap().as_str().to_owned();
	let output_filename = output_dir.join(basename.clone() + ".html");
    
    let mut out = read_to_string(&template).unwrap();

    for (key, value) in map.into_iter() {
        out = out.replace(key, value.as_str());
    }

	do_c(&mut out, &basename, &lib_dir, &include_dir, &c_dir);

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
