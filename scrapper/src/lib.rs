use std::borrow::Cow;
use anyhow::{anyhow, Error, Result};
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

type EchaData = Vec<HashMap<String, String>>;

/// Represents the subsections of the Composition(s) section:
/// - Boundary Composition(s) as `Subsection::Boundary`
/// - Legal Entity Composition(s) as `Subsection::LegalEntity`
/// - Composition(s) generated upon use as `Subsection::Generated`
/// - Other types of composition(s) as `Subsection::Other`
///
/// # Example
///     let boundary = Subsection::Boundary;
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub enum Subsection {
    Boundary,
    LegalEntity,
    Generated,
    Other,
}

impl FromStr for Subsection {
    type Err = Error;
    /// Parses a `string` to return an [`Subsection`] enum value.<br>
    /// <br>
    /// If parsing succeeds, return the value inside Ok, otherwise when the string is ill-formatted
    /// return an error specific to the inside Err. The error type is specific to the implementation of the trait.
    /// <br>
    /// # Example
    ///     assert_eq!(Subsection::Boundary, Subsection::from("Boundary Composition(s)")?)
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

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub enum Section {
    Identification,
    Composition(Subsection),
}

/// Represents and manages the data for each section and subsection of the provided url.
#[derive(Debug)]
pub struct EchaSite<'a> {
    // url: &'a str,
    url: Cow<'a, str>,
    data: HashMap<Section, EchaData>,
    document: Result<Html>,
}

/// Fetch the html body of the provided url.
fn fetch_document(url: &str) -> Result<Html> {
    println!("Fetching data from {url}...");
    let now = Instant::now();
    let body = reqwest::blocking::get(url)?.text()?;
    let elapsed = now.elapsed().as_secs();
    println!("\tFetched in: {} seconds", elapsed);

    Ok(Html::parse_document(&body))
}

/// Scrap the data of each constituent from the provided section.
/// <br>
/// The result is a vector where each element represents a panel from a section/subsection as a vector.
/// Each element of the latter is a HashMap with the data of the constituent.
fn data_from(document: &Html, section: Section) -> Result<EchaData> {
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

        Ok(vec![obtain_data(id_html).expect("Couldn't obtain Identification data")])
        // let wrap = vec![obtain_data(id_html).expect("Couldn't obtain Identification data")];
        // Ok(vec![wrap])
    };

    // **************************************************
    //*** Get subsection and panels data
    let panels_selector =
        Selector::parse("div.panel-group > h4 ,div.panel.panel-default").expect("panels_selector not created");
    let block_selector = Selector::parse("div.sBlock").expect("block_selector not created");
    let title_selector = Selector::parse("h4.panel-title").expect("title_selector not created");

    // **************************************************
    //*** Closure to get data from Compositions section
    let compositions_data = |subsection_enum| -> Result<EchaData> {
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

                    *state =
                        Section::Composition(Subsection::from_str(subsection.as_str()).ok()?);
                }

                Some((*state, node))
            })
            .filter(|(_, node)| node.value().name() != "h4");

        // Obtain constituents data of current panel
        let constituent_data: EchaData = sorted_panels_data
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
            .flatten()
            .collect();

        Ok(constituent_data)
    };

    match section {
        Section::Identification => id_data(),
        Section::Composition(sub) => compositions_data(Section::Composition(sub)),
    }
}

impl<'a> EchaSite<'a> {
    /// Create a new instance of the structure and fetch the html body
    /// from the provided url using [`fetch_document`].
    pub fn new(url: &'a str) -> Self {
        EchaSite {
            url: Cow::from(url),
            data: HashMap::default(),
            document: fetch_document(&url),
        }
    }

    pub fn get_constituents(&mut self, section: Section) -> EchaData {
        match self.data.get(&section) {
            Some(data) => data.clone(),
            None => {
                let document = match &self.document {
                    Ok(doc) => doc,
                    Err(error) => panic!("Couldn't obtain html body {error:?}"),
                };

                let new_data = data_from(document, section).unwrap();
                self.data.insert(section, new_data.clone());
                new_data
            }
        }
    }

}
