use anyhow::{anyhow, Result};
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

type EchaData = Vec<Vec<HashMap<String, String>>>;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum Subsection {
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
pub enum Section {
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

pub fn data_from(url: &str, section: Section) -> Result<EchaData> {
    let document = fetch_document(url.to_string())?;
    //
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
    let id_data = || -> Result<EchaData> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
