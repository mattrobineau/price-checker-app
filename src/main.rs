use notify_rust::Notification;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::fmt;

#[derive(Serialize, Deserialize)]
struct ProductDetail {
    price: f32,
    product_name: String,
    product_url: String,
    store_key: String,
}

#[derive(Serialize, Deserialize)]
struct StoreTemplate {
    attr: Option<String>,
    from_attr: bool,
    store_key: String,
    selector: String,
}

#[derive(Serialize, Deserialize)]
struct Root {
    products: Vec<ProductDetail>,
    stores: Vec<StoreTemplate>,
}

#[derive(Debug)]
struct SelectorParseError {
    details: String,
}

impl SelectorParseError {
    fn new(msg: &str) -> SelectorParseError {
        SelectorParseError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for SelectorParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for SelectorParseError {
    fn description(&self) -> &str {
        &self.details
    }
}

async fn get_price(
    product: &ProductDetail,
    store: &StoreTemplate,
    rg: &Regex,
) -> Result<f32, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 6.1; WOW64; rv:77.0) Gecko/20190101 Firefox/77.0")
        .build()?;

    let response = client
        .get(&product.product_url)
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&response);

    let selector = match Selector::parse(&store.selector) {
        Ok(s) => s,
        Err(_e) => {
            return Err(Box::new(SelectorParseError::new(
                format!("Error parsing selector \"{}\"", store.selector).as_str(),
            )))
        }
    };
    let element = document
        .select(&selector)
        .next()
        .expect("error in selector");

    let mut price = match store.from_attr {
        true => element
            .value()
            .attr(store.attr.as_ref().expect("no attribute set"))
            .expect(&format!(
                "no attribute named {} found in element {}",
                store.attr.as_ref().unwrap(),
                store.selector
            ))
            .to_string(),
        false => element.inner_html(),
    };

    let parsed_price = match rg.find(&price) {
        Some(p) => p.as_str(),
        None => {
            eprintln!("regex did not find a match in {}", &price);
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
    let json = tokio::fs::read_to_string("config.json").await?;

    let root: Root = serde_json::from_str(&json)?;

    let rg = Regex::new(r"[\d+,]*\.\d+")?;
    for product in &root.products {
        let store = match root
            .stores
            .iter()
            .find(|s| s.store_key == product.store_key)
        {
            Some(s) => s,
            None => continue,
        };

        let price = match get_price(&product, &store, &rg).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        if price < product.price {
            Notification::new()
                .summary("Price Alert")
                .body(
                    format!(
                        "{} has a lower price. Set price {:.2}, New {:.2}",
                        product.product_name, product.price, price
                    )
                    .as_str(),
                )
                .show()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! get_price_test {
        ($url:expr, $store:expr, $selector:expr, $price:expr, $from_attr:expr, $attr:expr) => {{
            let product = ProductDetail {
                price: 0.0,
                product_name: "not_used".to_string(),
                product_url: $url,
                store_key: $store,
            };

            let store = StoreTemplate {
                from_attr: $from_attr,
                attr: $attr,
                store_key: $store,
                selector: $selector,
            };

            let rg = Regex::new(r"[\d+,]*\.\d+").unwrap();
            let fetched_price = match get_price(&product, &store, &rg).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    0.00f32
                }
            };

            assert_eq!($price, fetched_price)
        }};
    }

    // #[tokio::test]
    // async fn get_price_from_thebrick() {
    //     get_price_test!(
    //         "https://www.thebrick.com/products/kate-nightstand".to_string(),
    //         "thebrick".to_string(),
    //         "#productPrice".to_string(),
    //         279.00f32,
    //         false,
    //         None
    //     )
    // }
}
