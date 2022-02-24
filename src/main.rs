use anyhow::{anyhow, Context, Result};
use directories::UserDirs;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use std::time::Instant;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
enum Subsection {
    Boundary,
    LegalEntity,
    Generated,
    Other,
}

impl FromStr for Subsection {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Subsection> {
        match input {
            "Boundary Composition(s)" => Ok(Subsection::Boundary),
            "Legal Entity Composition(s)" => Ok(Subsection::LegalEntity),
            "Composition(s) generated upon use" => Ok(Subsection::Generated),
            "Other types of composition(s)" => Ok(Subsection::Other),
            _ => Err(anyhow!("Couldn't parse {input} to Subsection Enum")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
enum Section {
    Identification,
    Composition(Subsection),
}

fn fetch_document(url: String) -> Result<Html> {
    println!("Fetching data...");
    let now = Instant::now();
    let body = reqwest::blocking::get(url)?.text()?;
    let elapsed = now.elapsed().as_secs();
    println!("\tTime elapsed fetching: {}", elapsed);

    Ok(Html::parse_document(&body))
}

fn data_from(document: &Html, section: Section) -> Result<Vec<Vec<HashMap<String, String>>>> {
    // **************************************************
    //*** Closure to obtain data from a sBlock
    let obtain_data = |data_html: ElementRef| -> Result<HashMap<String, String>> {
        // Useful selectors
        let dt_selector = Selector::parse("dt").unwrap();
        let dd_selector = Selector::parse("dd").unwrap();
        let img_selector = Selector::parse("img").unwrap();
        let constituent = Selector::parse("h5").unwrap();

        let key_names = data_html // Get key names
            .select(&dt_selector)
            .flat_map(|data| data.text())
            .map(|key| key.replace(":", ""));

        let key_values = data_html // Get key values
            .select(&dd_selector)
            .map(|data| {
                data.text()
                    .collect::<String>()
                    .trim()
                    .replace("\n", "")
                    .replace("\t", "")
            });

        let img = data_html.select(&img_selector).map(|link| {
            // Get image link if exists
            (
                String::from("Image link"),
                String::from(link.value().attr("src").unwrap_or("")),
            )
        });

        let consti_num = data_html
            .select(&constituent)
            .flat_map(|consti| consti.text())
            .map(|consti| {
                (
                    String::from("Constituent"),
                    String::from(consti.split(' ').last().unwrap()),
                )
            });

        Ok(key_names // Merge key names with key values
            .zip(key_values)
            .chain(img)
            .chain(consti_num)
            .collect())
    };

    // **************************************************
    //*** Closure to get data from Identification section
    let id_data = || -> Result<Vec<Vec<HashMap<String, String>>>> {
        let id_selector = Selector::parse("#sIdentification + div.sBlock").unwrap();
        let id_html = document // Get html info
            .select(&id_selector)
            .next()
            .expect("Problem obtaining identification html");

        let wrap = vec![obtain_data(id_html).expect("Couldn't obtain Identification data")];
        Ok(vec![wrap])
    };

    // **************************************************
    //*** Get subsection and panels data
    let panels_selector = Selector::parse("div.panel-group > h4 ,div.panel.panel-default").unwrap();
    let block_selector = Selector::parse("div.sBlock").unwrap();
    let title_selector = Selector::parse("h4.panel-title").unwrap();

    // **************************************************
    //*** Closure to get data from Compositions section
    let compositions_data = |subsection_enum| -> Result<Vec<_>> {
        // let subsection_enum = Section::Composition(Subsection::from(subsection));
        // Sort each panel to know which subsection it belongs to. Returns an iterator containing tuples (Subsection, Node)
        // Each panel has x constituents
        // h4 headers -> Subsections and the title of each listing item
        let sorted_panels_data = document
            .select(&panels_selector)
            .scan(Section::Composition(Subsection::Other), |state, node| {
                let kind = node.value().name();

                if kind == "h4" {
                    let subsection = node
                        .text()
                        .map(|e| e.trim().replace("\n", "").replace("\t", ""))
                        .collect::<String>()
                        .replace("open allclose all", "");

                    *state = Section::Composition(Subsection::from_str(subsection.as_str()).ok()?);
                }

                Some((*state, node))
            })
            .filter(|(_, node)| node.value().name() != "h4");

        // Obtain constituents data of current panel
        let constituent_data: Vec<_> = sorted_panels_data
            .filter(|(subsection, _)| *subsection == subsection_enum)
            // .inspect(|x| println!("Constituent {:?} {:?}", x.0, x.1.value().name()))
            .map(|(_, node)| {
                // Get the current panel title
                let panel_title: String = node
                    .select(&title_selector)
                    .flat_map(|e| e.text())
                    .map(|title| title.trim())
                    .filter(|title| !title.is_empty())
                    // .inspect(|t| eprintln!("t = {:#?}", t))
                    .collect();

                node.select(&block_selector)
                    .map(|constituent| obtain_data(constituent).unwrap())
                    .map(|mut data| {
                        data.insert("Name".to_string(), panel_title.to_string());
                        data
                    })
                    .collect::<Vec<HashMap<String, String>>>()
            })
            .collect();

        Ok(constituent_data)
    };

    match section {
        Section::Identification => id_data(),
        Section::Composition(sub) => compositions_data(Section::Composition(sub)),
        // _ => Err(anyhow!("Couldn't find match for section: {section:?}")),
    }
}

fn main() -> Result<()> {
    // **************************************************
    // ************ CLI args requirements ***************
    // **************************************************
    let url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("No CLI URL provided, using default.");
            "https://echa.europa.eu/registration-dossier/-/registered-dossier/24529".into()
            //"https://echa.europa.eu/registration-dossier/-/registered-dossier/26453".into()
        }
    };

    // **************************************************
    // ************ Create folder & files ***************
    // **************************************************
    if let Some(user_dirs) = UserDirs::new() {
        let output_dir = user_dirs
            .desktop_dir()
            .map(|path| path.join("echars_output"))
            .context("Couldn't create output folder path")?;

        // Output file name is the number of the dossier in the url.
        // The file is truncated if it already exists.
        let dossier = url
            .split('/')
            .last()
            .expect("Couldn't obtain dossier number");

        let _ofile_path = output_dir.join(dossier).with_extension("tsv");

        fs::create_dir_all(output_dir)?; //.expect("Couldn't create output folder path");
    }

    // **************************************************
    //*************** Fetching web page *****************
    // **************************************************
    let document = fetch_document(url)?;

    // **************************************************
    // **************** Getting data ********************
    // **************************************************
    let _identification = data_from(&document, Section::Identification)?;
    let _boundary = data_from(&document, Section::Composition(Subsection::Boundary))?;
    let _legal = data_from(&document, Section::Composition(Subsection::LegalEntity))?;
    let generated = data_from(&document, Section::Composition(Subsection::Generated))?;
    let _other = data_from(&document, Section::Composition(Subsection::Other))?;

    println!("{:#?}", generated);

    Ok(())
}
