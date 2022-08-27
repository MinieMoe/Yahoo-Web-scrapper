use std::collections::{HashSet, HashMap};
use std::f32::consts::E;
use std::hash::Hash;
use std::str::Lines;
use std::fs::File;
use std::rc::Rc;
use reqwest::blocking::Response;
use reqwest::{Request, Client};
use scraper::Html;
use select::document::{Document, self};
use select::predicate::{Attr, Name};
use url::Url;
use serde::{Serialize, Deserialize};

/*The scraper program will be given one argument, like coil
, which is the name of the website to scrape.
Website scraping is a recursive process.

1.	Initialize a list of all visited pages to be empty.
2.	Initialize a list of all downloaded images to be empty.
3.	Start with the URL of the home page.
4.	To visit a page
    a.	If the page has already been visited, return.
    b.	Otherwise use reqwest to download the HTML page at the given URL
    c.	For each page:
        i.	Keep track of the size of the page (this is given by the server in the header under `Content-Length`.
        ii.	Keep track of all links on the page so long as they point to the same domain, i.e. track only yahoo.com links, and discard everything else.
        iii.	Keep track of all images on the page
            1.	If the image has already been downloaded, continue.
            2.	Otherwise for each image, download the image and keep track of how large it is.
            3.	Add the image to the list of downloaded images
        iv.	Add the page to the list of visited pages
5.	Recursively visit each of the links on the page until there are no more pages
6.	Write the data you have collected about all the pages and all the images to a file that you can read later. You can use any format you want, e.g. CSV

*/

/* code dumps
//remove visited pages
    let new_urls = found_urls.difference(&visited)
    .map(|s| s.to_string())
    .collect::<HashSet<String>>();

    //add the new urls to the visited list now that we just visit them
    visited.extend(new_urls);
 */
#[derive(Serialize, Deserialize, Debug)]
 struct Page {
    size: usize,
    links: Vec<String>,  //list of all website urls found
    images: Vec<String>, //list of all images urls found
 }
 #[derive(Serialize, Deserialize, Debug)]
 struct Image{
    size: usize,
 }

 impl Page {
    fn new(size: usize, links: Vec<String>, images:Vec<String> ) -> Self{
        Self { size, links, images}
    }

    //get method for list of urls found on a page
    fn get_urls(&mut self) -> & Vec<String>{
        &self.links
    }
 }

 impl Image {
    fn new(size: usize) -> Image{
        Self {size}
    }
 }

 /* some URLs extracted from yahoo doesn't have https:// in front, so reqwest won't work on them 
    so we have to fix url before calling requwest on them
    add https:// header to some urls that dont have it so reqwest can work on them

    also, there're may be links that go outside of yahoo. ie: facebook page of yahoo
    we need to eliminate them

    We will use this function inside filter_map() to filter out these 2 kinds of URL (no https and not yahoo related)
    filter_map() takes Option<> as an arg so filter_url() has to return this type
    */
fn filter_url(link: &str) -> Option<String>{
    let url = Url::parse(link);
    match  url {
        //if the url is valid, aka has https:// then check if it points to yahoo.com
        Ok(url) =>{
            if url.has_host() && url.host_str().unwrap().ends_with("yahoo.com") && !url.to_string().contains("beap.gemini"){       //points to yahoo
                Some(url.to_string())
            }else{ // discard if not yahoo-related
                None
            }
        },
        //if the url is not valid, add https:// to it so it can used with reqwest
        Err(_e) =>{
            if link.starts_with("/"){//..or ends with .html
                Some(format!("https://yahoo.com{}",link))
            }else{//..not even a link, ex: javascript:void(0)
                None
            }
        }
    }
}

//discard any invalid image url
fn filter_img_url(link: &str) -> Option<String>{
    if link.contains("https://s.yimg.com") {
        Some(link.to_string())
    }else {
        None
    }
}

//send http request to the url and receive response. Return html in string and the size of the page in bytes
fn http_requester(link: &str) -> Option<String>{

    let client = reqwest::blocking::Client::new();
    let response = client.get(link)
    .header("User-Agent", "Mozilla/5.0");

    //had to manually handle error in case we get 404 url, which will make the program crash if we just use unwrap()
    match response.send() {
        Ok(rep) =>{
            Some(rep.text().unwrap())
        },
        Err(_e) =>{
            None
        }
    }
}


//extract urls from the given html
//change to Option<Vec<String>>? in case there's no link at all in a page???
fn extract_urls(html: &str) -> Vec<String>{
    //form a html document
    let document = Document::from(html);

    //extracting all links in the yahoo page and filter out bad urls
    //NOTE: use HashMap to avoid duplicate value, aka visted pages
    let found_urls= document.find(Name("a"))
    .filter_map(|node| node.attr("href"))
    .filter_map(|link| filter_url(link))
    .collect();    

    return found_urls;
}

//extracting all images from a page
fn extract_images(html: &str) -> Vec<String>{
    let document = Document::from(html);
    
    let found_images = document.find(Name("img"))
    .filter_map(|node| node.attr("src"))
    .filter_map(|link| filter_img_url(link))
    .collect();

    return found_images;
}

/*
    given a list of image urls, check if it's downloaded aka is it in 'downloaded' vector?
        if it's not:
            download the image to a folder
            retrieve size of image once downloaded
            make a new Image() and add to 'downloaded'
    add to the list of found images in a page (regardless of whether it was downloaded before or not)
 */
fn download_img(img_urls: &Vec<String>, downloaded: &mut HashMap<String, Image>){
    for img in img_urls{
        if !downloaded.contains_key(img){

            //"download" the image
            let img_bytes = reqwest::blocking::get(img).unwrap().bytes().unwrap();

            //get size of image just downloaded and update the downloaded list
            let size = img_bytes.len();
            downloaded.insert(img.to_string(), Image::new(size));

            //testing
            println!("Img: {}, Size: {}", img, size);

        }
    }
}


/* 
    check if the current link has been visited
        if visited, return
    if not visted,
        fetch html document via https request
        mark as visted 
        extract all links on the current url
    recursively scrap each links in the current url
        using for loop?
        stop recursion when there's no more link to go to
    
*/
fn recursive_scraper(link: &str, visited: &mut HashMap<String,Rc<Page>>, downloaded: &mut HashMap<String, Image>){
    if !visited.contains_key(link){
        
        let res = http_requester(link);
        
        if res.is_none(){//ignore invalid url 404
            return;
        }

        //scrap urls and imgs on a page
        let res_text = res.unwrap();
        let found_urls = extract_urls(&res_text);
        let found_imgs = extract_images(&res_text);
        let size = res_text.len();

        //printing links in hashmap, should NOT have dups
        println!("URL:{}; Size:{}", link, size);

        //download all images found
        println!("*******Images found within this link*******");
        download_img(&found_imgs, downloaded);

        let new_page = Rc::new(Page::new(size, found_urls, found_imgs));
        visited.insert(link.to_string(), new_page.clone());


        for url in &new_page.links {
            recursive_scraper(&url,visited, downloaded);
        }
    }

    return;

}

fn main() {
    
    //list of visited website
    let mut visited: HashMap<String, Rc<Page>> = HashMap::new();
    //list of downloaded images
    let mut downloaded: HashMap<String, Image> = HashMap::new();

    //file to write result to
    let pages_path = "visited.json";
    let imgs_path = "downloaded.json";

    let pages_file = File::create(pages_path).unwrap();
    let imgs_file = File::create(imgs_path).unwrap();

    //fetching the url from the user: need to start with http:/ or https:/
    let url = std::env::args().last().unwrap();
    let http_head = &(url.as_str())[..4];
    
    if http_head.ne("http"){
        print!("Not URL!");
        return;
    }
    
    recursive_scraper(&url, &mut visited, &mut downloaded,);
    let pages_cereal =  serde_json::to_string(&visited).unwrap();
    let imgs_cereal = serde_json::to_string(&downloaded).unwrap();

    serde_json::ser::to_writer(pages_file, &pages_cereal).unwrap();
    serde_json::ser::to_writer(imgs_file, &imgs_cereal).unwrap();


}

/*
serde to serialize data
pull request 
*/