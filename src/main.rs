use std::env::{current_exe, set_current_dir, args};
use std::fs::{create_dir, remove_file, remove_dir_all, read_to_string, write};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;
use dateparser::parse;
use chrono::{Datelike, DateTime, Utc};
use glob::glob;
use regex::Regex;
use uuid::Uuid;
use rayon::prelude::*;
use serde::{Serialize, Deserialize};
use toml;

#[derive(Serialize, Deserialize)]
struct Config<'a> {
    cc:             Cow<'a, str>,
    im:             Cow<'a, str>,

    lib_dir:        Cow<'a, Path>,
    include_dir:    Cow<'a, Path>,
    c_dir:          Cow<'a, Path>,
    content_dir:    Cow<'a, Path>,
    output_dir:     Cow<'a, Path>,
    copy_year:      i32,
    root_url:       Cow<'a, str>,
    thumbnails_dir: Cow<'a, str>,
    template_fn:    Cow<'a, str>,
    default_thumb:  Cow<'a, str>,
    prelude_path:   Cow<'a, Path>,
    font_fn:        Cow<'a, str>,
    docroot_dir:    Option<Cow<'a, str>>,

    site_name:      Cow<'a, str>,
    site_desc:      Cow<'a, str>,
    rss_path:       Cow<'a, Path>,
}

fn do_c(html: &mut String, basename: &str, config: &Config, c_prelude: &str) {
    let c_re = Regex::new(r"(?s)<c>(.*?)</c>").unwrap();
    while let Some(capture) = c_re.captures(&html) {
        let source_match = capture.get(1).unwrap();

        let source = format!(r#"{}
            #define THIS_FILE "content/{}.md"
            #define THIS_BASENAME "{}"
            int main(void) {{
                {}
                return 0;
            }}"#,
            c_prelude,
            basename,
            if basename == "index" { "home" } else { basename },
            source_match.as_str());

        let id = Uuid::new_v4();

        let c_fn = config.c_dir.join(format!("src_{basename}_{id}.c"));
        write(&c_fn, source).unwrap();

        let o_fn = config.c_dir.join(format!("out_{basename}_{id}"));
        let out = match Command::new(config.cc.as_ref()).
            arg(c_fn)
            .args(glob(config.lib_dir.join("*.o").to_str().unwrap()).unwrap().map(|p| p.unwrap()))
            .arg("-lm")
            .arg("-I")
            .arg(config.include_dir.as_ref())
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
	let typertags_re = Regex::new(r#"\[(\w+)](?:\s+)?\{([\S\s]*?)\n\}"#).unwrap();
	let code_re = Regex::new(r"```.*?```").unwrap();

    let mut cit = code_re.find_iter(contents);
    let mut closest_code_block = cit.next();
    let mut replacements: Vec<(core::ops::Range<usize>, String)> = vec![];
    'captures: for capture in typertags_re.captures_iter(&contents) {
        'advance_code: while let Some(code) = closest_code_block {
            let r = code.range();
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
        let (_, [tag_name, tag_body]) = capture.extract();

        let rep = match tag_name {
            "note" => {
                let mut html = String::new();
                pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&tag_body));
                format!(r#"<div class="side-note">{html}</div>"#)
            },
            _ => tag_body.to_owned(),
        };

        replacements.push((capture.get(0).unwrap().range(), rep));
    }

    for (range, content) in replacements.into_iter().rev() {
        contents.replace_range(range, &content);
    }
}

fn compose(
    filename: &PathBuf,
    contents: &str,
    config: &Config,
    template: &str,
    c_prelude: &str
) {
    let mut lines = contents.lines();
    let title = lines.next().unwrap()[2..].to_owned();
    let description = lines.next().unwrap()[2..].to_owned();
    let basename = Regex::new(r"(?i)(.*/)?([A-Za-z0-9_-]+)\.md").unwrap().captures(filename.to_str().unwrap()).unwrap().get(2).unwrap().as_str();
    lines.next();
    let thumb = match lines.next() {
        Some(line) if Regex::new(r"%\s").unwrap().is_match(line) => line[2..].to_owned(),
        _ => {
            let thumb = format!("{}{basename}.png", config.thumbnails_dir);
            format!("{}{}", config.root_url, match Command::new(config.im.as_ref())
                    .arg(config.default_thumb.as_ref())
                    .arg("(")
                    .arg("-size").arg("1200x320")
                    .arg("gradient:transparent-black")
                    .arg(")")
                    .arg("-gravity").arg("South")
                    .arg("-compose").arg("Over")
                    .arg("-composite")
                    .arg("(")
                    .arg("-fill").arg("white")
                    .arg("-background").arg("transparent")
                    .arg("-size").arg("1150x150")
                    .arg("-font").arg(config.font_fn.as_ref())
                    .arg("-gravity").arg("SouthWest")
                    .arg(format!("caption:{title}"))
                    .arg(")")
                    .arg("-gravity").arg("South")
                    .arg("-compose").arg("Over")
                    .arg("-composite")
                    .arg(config.output_dir.join(&thumb))
                    .status() {
                        Ok(_) => thumb.as_str(),
                        Err(_) => config.default_thumb.as_ref(),
                    }
            )
        },
    };

    println!("Composing {} (\"{}\")...", filename.display(), title);

    let mut r_contents = Regex::new(r"(?m)^%\s*.*").unwrap().replace_all(&contents, "").to_string();

    let head_capture = Regex::new(r"(?s)<\#inject_head\#>(.*?)</\#inject_head\#>").unwrap().captures(&r_contents);
    let (head_injection, head_range) = match head_capture {
        Some(head) => (head.get(1).unwrap().as_str().to_string(), Some(head.get(0).unwrap().range())),
        None => ("".to_string(), None),
    };

    if let Some(r) = head_range {
        r_contents.replace_range(r, "");
    }

    do_typer_tags(&mut r_contents);

    do_c(&mut r_contents, &basename, &config, c_prelude);

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&r_contents));

    let map = [
        ("`META_PAGE_TITLE`", html_escape::encode_safe(&title).to_string()),
        ("`PAGE_TITLE`", title),
        ("`META_PAGE_DESCRIPTION`", html_escape::encode_safe(&description).to_string()),
        ("`PAGE_DESCRIPTION`", description),
        ("`CONTENT`", html),
        ("`YEAR`", config.copy_year.to_string()),
        ("`META_THUMBNAIL`", thumb),
        ("`HEAD_INJECT`", head_injection)
    ];

    let output_filename = config.output_dir.join(format!("{basename}.html"));

    let mut out = template.to_owned();

    for (key, value) in map {
        out = out.replace(key, value.as_str());
    }

    do_c(&mut out, &basename, &config, c_prelude);

    write(output_filename, out).unwrap();
}

fn write_rss(
    config: &Config,
    pages: &Vec<(&PathBuf, String, DateTime<Utc>)>
) {
    println!("Writing {}...", config.rss_path.display());
    let out = format!(r#"<?xml version="1.0" encoding="UTF-8" ?>
    <rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
        <channel>
            <title>{}</title>
            <description>{}</description>
            <link>{}</link>
            <lastBuildDate>{}</lastBuildDate>
            <atom:link href="{}{}" rel="self" type="application/rss+xml" />
            
            {}
        </channel>
    </rss>"#,
    config.site_name,
    config.site_desc,
    config.root_url,
    chrono::offset::Utc::now().to_rfc2822(),
    config.root_url, config.rss_path.display(),
    pages.iter().rev().filter_map(|(fname, contents, time)|
        if time.timestamp() == 0 {
            None
        } else {
            let mut lines = contents.lines();
            let fns = fname.to_str().unwrap();
            let title = lines.next().unwrap();
            let desc = lines.next().unwrap();
            Some(format!(r#"
                <item>
                    <title>{}</title>
                    <description>{}</description>
                    <link>{}{}.html</link>
                    <guid>{}{}.html</guid>
                    <pubDate>{}</pubDate>
                </item>
            "#,
            &title[2..],
            &desc[2..],
            config.root_url, &fns[8..fns.len()-3],
            config.root_url, &fns[8..fns.len()-3],
            time.to_rfc2822()))
        }
    ).take(10).collect::<Vec<_>>().join("\n"));

    write(config.output_dir.join(config.rss_path.clone()), out).unwrap();
}

fn main() -> Result<(), std::io::Error> {
    let config = toml::from_str(read_to_string(args().skip(1).next().as_ref().map_or("compost.toml", String::as_str)).unwrap().as_str()).unwrap_or(Config {
        cc             : Cow::Borrowed("gcc"),
        im             : Cow::Borrowed("magick"),
        lib_dir        : Cow::Borrowed(Path::new("./lib/")),
        include_dir    : Cow::Borrowed(Path::new("./include/")),
        c_dir          : Cow::Borrowed(Path::new("./bin/")),
        content_dir    : Cow::Borrowed(Path::new("./content/")),
        output_dir     : Cow::Borrowed(Path::new("./out/")),
        copy_year      : chrono::Utc::now().year(),
        root_url       : Cow::Borrowed("https://lachrymal.net/"),
        thumbnails_dir : Cow::Borrowed("thumbnails/"),
        font_fn        : Cow::Borrowed("Helvetica"),
        template_fn    : Cow::Borrowed("template.html"),
        default_thumb  : Cow::Borrowed("default.png"),
        prelude_path   : Cow::Borrowed(Path::new("prelude.c")),
        docroot_dir    : None,
        site_name      : Cow::Borrowed("compost"),
        site_desc      : Cow::Borrowed("Welcome!"),
        rss_path       : Cow::Borrowed(Path::new("index.xml")),
    });

    let c_prelude = read_to_string(&config.prelude_path).expect("Could not find C prelude...");

    let dir = current_exe().unwrap().parent().unwrap().to_owned();
    set_current_dir(&dir).unwrap();

    println!("   ______                                 __
  / ____/___  ____ ___  ____  ____  _____/ /_
 / /   / __ \\/ __ `__ \\/ __ \\/ __ \\/ ___/ __/
/ /___/ /_/ / / / / / / /_/ / /_/ (__  ) /_
\\____/\\____/_/ /_/ /_/ .___/\\____/____/\\__/
                    /_/                      ");

    println!("Processing directory {}...", dir.display());
    println!("=============");

    _ = remove_dir_all(&config.output_dir);
    create_dir(&config.output_dir).expect("Could not create output directory for pages.");
    create_dir("./out/thumbnails").expect("Could not create output directory for thumbnails.");
    _ = remove_dir_all(&config.c_dir);
    create_dir(&config.c_dir).expect("Could not create output directory for C.");

    let fns: Vec<_> = glob(config.content_dir.join("*.md").to_str().unwrap()).unwrap().collect::<Vec<_>>();
    let mut pages: Vec<_> = fns.par_iter()
    .filter_map(|p| {
        match p {
            Ok(fname) => {
                match read_to_string(&fname) {
                    Ok(f) => {
                        let time = parse(&f.lines().nth(2).unwrap()[2..]).unwrap();
                        Some((fname, f, time))
                    }
                    _ => None
                }
            }
            _ => None
        }
    })
    .collect();

    pages.par_sort_by_key(|p| p.2);

    let template = read_to_string(config.template_fn.as_ref()).expect("The specified template could not be found...");
    
    let new_c_prelude = format!(r#"
        {}
        const char * CONTENT_FILES[] = {{ "{}" }};
        const uint8_t N_CONTENT_FILES = sizeof(CONTENT_FILES) / sizeof(char *);
        "#,
        c_prelude,
        pages.iter().map(|(p, _, _)| p.to_str().unwrap()).collect::<Vec<_>>().join("\",\"")
    );
    
    pages.par_iter().for_each(|(fname, page, _)| compose(fname, page.as_str(), &config, &template, new_c_prelude.as_str()));

    write_rss(&config, &pages);

    if let Some(dir) = config.docroot_dir {
        let path = Path::new(dir.as_ref());

        println!("Updating website...");
        println!("===================");

        for html in glob(path.join("*.html").to_str().unwrap())
            .expect("The specified directory could not be found...") {
            match html {
                Ok(f) => remove_file(f).unwrap(),
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Command::new("rsync")
                .arg("-avh")
                .arg(config.output_dir.as_ref())
                .arg(path)
                .spawn()
                .unwrap();
    }

    Ok(())
}
