use regex::Regex;
use scraper::{ Html, Selector };

struct ProductDetail {
    price: u32,
    product_url: String,
    store_key: String,
}

struct StoreTemplate {
    store_key: String,
    selector: String,
}

async fn get_price(product: &ProductDetail, store: &StoreTemplate, rg: &Regex) -> Result<f32, Box<dyn std::error::Error>> {
    let response = reqwest::get(&product.product_url).await?.text().await?;

    let document = Html::parse_document(&response);
    let selector = Selector::parse(&store.selector).unwrap();

    let element = document.select(&selector).next().unwrap();
    let mut price = element.inner_html();

    let parsed_price = match rg.find(&price) {
        Some(p) => p.as_str(),
        None => {
            println!("regex did not find a match in {}", &price);
            "0.00"
        }
    };
    price = parsed_price.replace(",", "");

    match price.trim().parse::<f32>() {
        Ok(value) => Ok(value),
        Err(error) => Err(Box::new(error)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_stores = vec![StoreTemplate {
        store_key: "redbubble".to_string(),
        selector: String::from("div[class^=DesktopProductPage__config] span span"),
    }];

    let sample_products = vec![ProductDetail {
            price: 32,
            product_url: String::from("https://www.redbubble.com/i/sweatshirt/The-Bodacious-Period-by-wytrab8/26255784.73735"),
            store_key: String::from("redbubble"),
        }];

    let rg = Regex::new(r"[\d+,]*\.\d+").unwrap();
    for product in &sample_products {
        let store = match sample_stores.iter().find(|s| s.store_key == product.store_key) {
            Some(s) => s,
            None => continue,
        };

        let price = get_price(&product, &store, &rg).await?;
        println!("{}", price);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_price_from_site() {
        let product = ProductDetail {
            price: 32,
            product_url: String::from("https://www.redbubble.com/i/sweatshirt/The-Bodacious-Period-by-wytrab8/26255784.73735"),
            store_key: String::from("redbubble"),
        };

        let store = StoreTemplate {
            store_key: String::from("redbubble"),
            selector: String::from("div[class^=DesktopProductPage__config] span span"),
        };

        let rg = Regex::new(r"[\d+,]*\.\d+").unwrap();
        let x = match get_price(&product, &store, &rg).await {
            Ok(p) => p,
            Err(e) => {
                println!("{}", e);
                0.00f32
            }
        };

        assert_eq!(x, 55.31f32)
    }
}
