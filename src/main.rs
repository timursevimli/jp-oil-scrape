use csv::Writer;
use once_cell::sync::Lazy;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut product_urls = Vec::new();
    for url in URLS {
        let body = get_body(&client, url).await?;
        let products = get_products_urls(Html::parse_document(&body));
        product_urls.extend(products);
    }
    let mut products = Vec::new();
    for url in &product_urls {
        let body = get_body(&client, url).await?;
        let product = parse_product(&body);
        products.push(product);
    }
    println!("CSV creating...");
    create_csv(&products)?;
    println!("Downloading files...");
    for product in products {
        match try_join!(
            download_file(&client, &product.image_url, &product.title),
            download_file(&client, &product.tds_form_pdf_url, &product.title),
            download_file(&client, &product.msds_form_pdf_url, &product.title),
        ) {
            Ok(_) => println!("Successfully downloaded files for {}", product.title),
            Err(e) => {
                eprintln!("Error downloading files for {}", product.title);
                eprintln!("Image URL: {}", product.image_url);
                eprintln!("TDS URL: {}", product.tds_form_pdf_url);
                eprintln!("MSDS URL: {}", product.msds_form_pdf_url);
                eprintln!("Error: {:?}", e);
                continue;
            }
        }
    }
    println!("Done!");
    Ok(())
}

fn create_csv(products: &[Product]) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = Writer::from_path("products.csv")?;

    wtr.write_record([
        "ID",
        "Type",
        "SKU",
        "GTIN, UPC, EAN, or ISBN",
        "Name",
        "Published",
        "Is featured?",
        "Visibility in catalog",
        "Short description",
        "Description",
        "Date sale price starts",
        "Date sale price ends",
        "Tax status",
        "Tax class",
        "In stock?",
        "Stock",
        "Low stock amount",
        "Backorders allowed?",
        "Sold individually?",
        "Weight (lbs)",
        "Length (in)",
        "Width (in)",
        "Height (in)",
        "Allow customer reviews?",
        "Purchase note",
        "Sale price",
        "Regular price",
        "Categories",
        "Tags",
        "Shipping class",
        "Images",
        "Download limit",
        "Download expiry days",
        "Parent",
        "Grouped products",
        "Upsells",
        "Cross-sells",
        "External URL",
        "Button text",
        "Position",
        "Brands",
    ])?;

    for (index, product) in products.iter().enumerate() {
        let id = 296 + index;

        wtr.write_record(&[
            id.to_string(),                                                         // ID
            "simple".to_string(),                                                   // Type
            format!("SKU{}", id),                                                   // SKU
            "".to_string(),                                                         // GTIN
            product.title.clone(),                                                  // Name
            "1".to_string(),                                                        // Published
            "0".to_string(),                                                        // Is featured?
            "visible".to_string(),                                                  // Visibility
            product.short_description.clone(), // Short description
            product.description.clone(),       // Description
            "".to_string(),                    // Date sale price starts
            "".to_string(),                    // Date sale price ends
            "taxable".to_string(),             // Tax status
            "".to_string(),                    // Tax class
            "1".to_string(),                   // In stock?
            "10".to_string(),                  // Stock
            "".to_string(),                    // Low stock amount
            "0".to_string(),                   // Backorders allowed?
            "0".to_string(),                   // Sold individually?
            "".to_string(),                    // Weight
            "".to_string(),                    // Length
            "".to_string(),                    // Width
            "".to_string(),                    // Height
            "1".to_string(),                   // Allow customer reviews?
            "".to_string(),                    // Purchase note
            "15".to_string(),                  // Sale price
            "25".to_string(),                  // Regular price
            "Motor Yağları > Binek ve Hafif Ticari Araç Motor Yağları".to_string(), // Categories
            "motoryag, yag".to_string(),       // Tags
            "".to_string(),                    // Shipping class
            product.image_url.clone(),         // Images
            "".to_string(),                    // Download limit
            "".to_string(),                    // Download expiry days
            "".to_string(),                    // Parent
            "".to_string(),                    // Grouped products
            "".to_string(),                    // Upsells
            "".to_string(),                    // Cross-sells
            "".to_string(),                    // External URL
            "".to_string(),                    // Button text
            "0".to_string(),                   // Position
            "JAPAN OIL".to_string(),           // Brands
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

async fn download_file(
    client: &Client,
    url: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let safe_dir = path.replace(" ", "_");
    fs::create_dir_all(&safe_dir)?;
    let filename = url.split('/').last().unwrap();
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;
    fs::write(format!("{}/{}", safe_dir, filename), bytes)?;
    Ok(())
}

fn get_products_urls(document: Html) -> Vec<String> {
    let selector = Selector::parse(".elementor-image-box-title a").unwrap();
    document
        .select(&selector)
        .filter_map(|element| element.attr("href"))
        .map(String::from)
        .collect()
}

async fn get_body(client: &Client, url: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(client.get(url).send().await?.text().await?)
}

fn get_value(document: &Html, selector: Selector) -> String {
    document
        .select(&selector)
        .next()
        .map(|element| {
            if element.value().name() == "a" {
                element.value().attr("href").unwrap_or_default().to_string()
            } else if element.value().name() == "img" {
                element.value().attr("src").unwrap_or_default().to_string()
            } else {
                element.text().collect::<Vec<_>>().join("")
            }
        })
        .unwrap_or_default()
}

static SELECTORS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "title",
        "#content > div:nth-of-type(2) > div > section:nth-of-type(2) > \
          div > div:nth-of-type(2) > div > div > div > h2",
    );
    m.insert(
        "description",
        "#content > div:nth-of-type(2) > div > section:nth-of-type(2) > \
          div > div:nth-of-type(2) > div > section:first-of-type > div > \
          div > div > div > div > p",
    );
    m.insert(
        "tds_form_pdf_url",
        "#content > div:nth-of-type(2) > div > section:nth-of-type(2) > \
         div > div:nth-of-type(2) > div > section:nth-of-type(3) > div > \
         div:first-of-type > div > div:nth-of-type(3) > div > div > a",
    );
    m.insert(
        "msds_form_pdf_url",
        "#content > div:nth-of-type(2) > div > section:nth-of-type(2) > \
         div > div:nth-of-type(2) > div > section:nth-of-type(3) > div > \
         div:nth-of-type(2) > div > div:nth-of-type(3) > div > div > a",
    );
    m.insert(
        "image_url",
        "#content > div:nth-of-type(2) > div > section:nth-of-type(2) > \
         div > div:first-of-type > div > div > div > img",
    );
    m
});

fn get_selector(target: &str) -> Selector {
    let selector = SELECTORS.get(target).unwrap();
    Selector::parse(selector).unwrap()
}

fn parse_product(body: &str) -> Product {
    let document = Html::parse_document(body);

    let selector = get_selector("title");
    let title = get_value(&document, selector).trim_start().to_string();

    let selector = get_selector("description");
    let description = get_value(&document, selector).to_string();

    let selector = get_selector("image_url");
    let image_url = get_value(&document, selector).to_string();

    let selector = get_selector("tds_form_pdf_url");
    let tds_form_pdf_url = get_value(&document, selector).to_string();

    let selector = get_selector("msds_form_pdf_url");
    let msds_form_pdf_url = get_value(&document, selector).to_string();

    let description = format!(
            "{}\n\n<a href=\"{}\" target=\"_blank\" rel=\"noopener\">TDS FORMU</a>\n<a href=\"{}\" target=\"_blank\" rel=\"noopener\">MSDS FORMU</a>",
            description,
            tds_form_pdf_url,
            msds_form_pdf_url
        );

    let short_description = description.split('.').next().unwrap().to_string() + ".";

    Product {
        title,
        description,
        short_description,
        image_url,
        tds_form_pdf_url,
        msds_form_pdf_url,
    }
}

#[derive(Debug)]
struct Product {
    title: String,
    short_description: String,
    description: String,
    image_url: String,
    tds_form_pdf_url: String,
    msds_form_pdf_url: String,
}

static URLS: [&str; 6] = [
    "https://japanoil.jp/passenger-and-light-commercial-vehicle-engine-oils/",
    "https://japanoil.jp/heavy-commercial-vehicle-engine-oils/",
    "https://japanoil.jp/transmission-and-differential-oils/",
    "https://japanoil.jp/motorcycle-oils/",
    "https://japanoil.jp/brake-fluids/",
    "https://japanoil.jp/lubricating-greases/",
];
