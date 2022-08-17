use std::collections::{HashSet, HashMap};
use std::hash::Hash;
use std::io::Read;
use std::str::Lines;
//use std::str::Bytes;
use reqwest::blocking::Response;
use reqwest::{Request, Client, header::{HeaderMap, HeaderValue}};
use scraper::Html;
use select::document::{Document, self};
use select::predicate::{Attr, Name};
use url::Url;
use bytes::{Bytes, BytesMut, Buf, BufMut};

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

/* 
Get your code in a branch under tools/scraper (you called it scrapper) code reviewed and checked in
For every page, store
    The size of the page
    The links on the page (filter out all links that map to somewhere else)
    The images on the page
For every image
    The size of the image
Might want to use a HashMap
Store the hash map as the output, so we can read the hashmap back in again
A follow on for the fall quarter is write a server that reads the hashmap and serves yahoo.
 */
 struct Page {
    size: usize,
    links: Vec<String>,  //list of all website urls found
    images: Vec<String>, //list of all images urls found
 }

 struct Image{
    size: u64,
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
    fn new(size: u64) -> Image{
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

//send http request to the url and receive response. Return html in string and the size of the page in bytes
/*
fn http_requester(link: &str) -> Option<Result<bytes::Bytes, reqwest::Error>>{

    let client = reqwest::blocking::Client::new();
    let response = client.get(link)
    .header("User-Agent", "Mozilla/5.0");
    //.send().map(|res| println!("{:?}", res));

    //had to manually handle error in case we get 404 url, which will make the program crash if we just use unwrap()
    match response.send() {
        Ok(rep) =>{
            //Some((get_size(&rep), rep.text().unwrap()))
            Some(rep.bytes())
        },
        Err(_e) =>{
            None
        }
    }
}
*/
fn http_requester(link: &str) -> Option<String>{

    let client = reqwest::blocking::Client::new();
    let response = client.get(link)
    .header("User-Agent", "Mozilla/5.0");
    //.send().map(|res| println!("{:?}", res));

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

    //TODO: downoad html document

    //extracting all links in the yahoo page and filter out bad urls
    //NOTE: use HashSet to avoid duplicate value, aka visted pages
    let found_urls= document.find(Name("a"))
    .filter_map(|node| node.attr("href"))
    .filter_map(|link| filter_url(link))
    .collect();

    return found_urls;
}

//extracting all images from a page
fn extract_images(html: &str) -> Vec<String>{
    let document = Document::from(html);
    
    //TODO: download images

    let found_images = document.find(Name("img"))
    .filter_map(|node| node.attr("src"))
    .map(|i| i.to_string())
    .collect();

    return found_images;
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
fn recursive_scraper(link: &str, visited: &mut HashMap<String, Page>){
    if !visited.contains_key(link){
        
        let res = http_requester(link);
        
        if res.is_none(){//ignore invalid url 404
            return;
        }

        let res_text = res.unwrap();

        let found_urls = extract_urls(&res_text);
        let found_imgs = extract_images(&res_text);
        let size = res_text.len();

        let new_page = Page::new(size, found_urls, found_imgs);
        
        /*Accessing links of Page directly. Error: new_page was moved into list before inserting it into visited
        let list = &new_page.links;

        visited.insert(link.to_string(), new_page);

        println!("url:{}; size:{}", link, size);//printing links in hashmap, should NOT have dups

        for url in list {
            recursive_scraper(&url,visited);
        }

        */
        /*Accessing links of Page by accessing visited. Error: double mutable borrow
        visited.insert(link.to_string(), new_page);

        let list = &(visited.get_mut(link).unwrap().links);

        println!("url:{}; size:{}", link, size);//printing links in hashmap, should NOT have dups

        for url in list {
            recursive_scraper(&url,visited);
        }
        */
    }

    return;

}

fn main() {
    
    //list of visited website
    let mut visited: HashMap<String, Page> = HashMap::new();

    //fetching the url from the user: need to start with http:/ or https:/
    let url = std::env::args().last().unwrap();
    let http_head = &(url.as_str())[..4];
    
    if http_head.ne("http"){
        print!("Not URL!");
        return;
    }

    /*
        //sending http/https request to host: yahoo.com
        let res_text = http_requester(&url);

        //update the list of visited website, yahoo.com will be first
        visited.insert(url);

        //extracting all links in the MAIN yahoo page and filter out bad urls
        let found_urls = extract_urls(&res_text);

        print!("{:#?}", found_urls);
    */
    
    recursive_scraper(&url, &mut visited);

}
