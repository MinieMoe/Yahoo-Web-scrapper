use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use reqwest;
use select::document::{Document};
use select::predicate::{Name};
use url::Url;
use serde::{Serialize, Deserialize};

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
//if the response give error, tries the link again 3 time, if still fails, add to fail list
fn http_requester(link: &str, mut tries:u32, baddies: &mut Vec<String>) -> Option<String>{

    if tries == 4{
        baddies.push(link.to_string());
        return None;
    }

    let client = reqwest::blocking::Client::new();
    let response = client.get(link)
    .header("User-Agent", "Mozilla/5.0");

    //had to manually handle error in case we get 404 url, which will make the program crash if we just use unwrap()
    match response.send() {
        Ok(rep) =>{
            match rep.text(){
                Ok(txt) =>{
                    Some(txt)
                },
                Err(_e) =>{ //try the link 3 times then stop if still gives error
                    println!("Fail! {}", _e);
                    tries +=1;
                    http_requester(link, tries, baddies)
                }
            }
        },
        Err(_e) =>{
            println!("Fail! {}", _e);
            tries +=1;
            http_requester(link, tries, baddies)
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
fn download_img(img_urls: &Vec<String>, downloaded: &mut HashMap<String, Image>, baddies:&mut Vec<String>){
    for img in img_urls{
        if !downloaded.contains_key(img){

            println!("Processing...{}", img);

            //"download" the image
            //let img_bytes = reqwest::blocking::get(img).unwrap().bytes().unwrap();

            //TODO: check for error here instead of unwrap()
            match reqwest::blocking::get(img) {
                Ok(rep) => {
                    match rep.bytes() {
                        Ok(img_bytes) =>{
                            //get size of image just downloaded and update the downloaded list
                            let size = img_bytes.len();
                            downloaded.insert(img.to_string(), Image::new(size));
                            //testing
                            println!("Success! -> size: {}",size);
                        },
                        Err(_e) =>{
                            println!("Fail! {}", _e);
                            baddies.push(img.to_string());
                        }
                    }
                },
                Err(_e) =>{
                    println!("Fail! {}", _e);
                    baddies.push(img.to_string());
                }
            }
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
fn recursive_scraper(link: &str, visited: &mut HashMap<String,Rc<Page>>, downloaded: &mut HashMap<String, Image>, baddies: &mut Vec<String>){
    if !visited.contains_key(link){
        
        println!("Processing...{}", link);      //checking which link is being scraped in case it crashes

        let res = http_requester(link, 1, baddies);
        
        if res.is_none(){//ignore invalid url 404
            return;
        }

        //scrap urls and imgs on a page
        let res_text = res.unwrap();
        let found_urls = extract_urls(&res_text);
        let found_imgs = extract_images(&res_text);
        let size = res_text.len();

        //printing links in hashmap, should NOT have dups
        println!("Sucess! -> Size:{}", size);

        //download all images found
        println!("*******Images found within this link*******");
        download_img(&found_imgs, downloaded, baddies);

        //use Rc<Page> so we can share the page between 'visisted' and the recurive loop
        let new_page = Rc::new(Page::new(size, found_urls, found_imgs));
        visited.insert(link.to_string(), new_page.clone());


        for url in &new_page.links {
            recursive_scraper(&url,visited, downloaded, baddies);
        }
    }

    return;

}

fn main() {
    
    //list of visited website
    let mut visited: HashMap<String, Rc<Page>> = HashMap::new();
    //list of downloaded images
    let mut downloaded: HashMap<String, Image> = HashMap::new();
    //list of failed URLs
    let mut baddies: Vec<String> = Vec::new();

    //file to write results to
    let pages_file = File::create("visited.json").unwrap();
    let imgs_file = File::create("downloaded.json").unwrap();
    let fails_file = File::create("baddies.json").unwrap();

    //fetching the url from the user: need to start with http:/ or https:/
    let url = std::env::args().last().unwrap();
    let http_head = &(url.as_str())[..4];
    
    if http_head.ne("http"){
        print!("Not URL!");
        return;
    }
    
    recursive_scraper(&url, &mut visited, &mut downloaded, &mut baddies);

    //serialize result as JSON string to the created paths
    let pages_cerealizer = serde_json::ser::to_writer_pretty(pages_file, &visited).unwrap();
    let imgs_cerealizer = serde_json::ser::to_writer_pretty(imgs_file, &downloaded).unwrap();
    let fail_cerealizer = serde_json::ser::to_writer_pretty(fails_file, &baddies).unwrap();


}

/*
serde to serialize data
pull request 
*/