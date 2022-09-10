use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;
use reqwest;
use select::document::{Document};
use select::predicate::{Name};
use url::Url;
use serde::{Serialize, Deserialize};
use clap::{Command, Arg};

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
    let request = client.get(link)
    .header("User-Agent", "Mozilla/5.0")
    .timeout(Duration::new(3, 0));  //if the request sent is hung for more than 3 seconds, stop and return time out error

    let response = request.send();
    //println!("request sent!");

    //had to manually handle error in case we get 404 url, which will make the program crash if we just use unwrap()
    match response {
        Ok(rep) =>{
            match rep.text(){
                Ok(txt) =>{
                    //println!("got text");
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

            println!("Processing IMG...{}", img);

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


/* DEPRECATED
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

/*non-recursive bfs scraper
    local lists: found_urls -> list of urls found in a page, may or may not have been visited
    start with yahoo.com, add it to found_urls
    while found_urls is not empty, iterate through each link in the list starting from the front
        check if the current url has been visited
            if not, scrap each url in the list and add them to the found_url. 
                Download all the image on this page too
                Then add this url to list of visted website
            if vististed, then skip this url and move on to the next one on the list
*/
fn bfs_scraper(link: &str, visited: &mut HashMap<String,Rc<Page>>, downloaded: &mut HashMap<String, Image>, baddies: &mut Vec<String>, mut log_file:File){
    let mut found_urls: VecDeque<String> = VecDeque::new();
    found_urls.push_back(link.to_string());

    while !found_urls.is_empty(){
        let url = found_urls.pop_front().unwrap();

        println!("Processing URL...{}", url);      //checking which link is being scraped in case it crashes

        let res = http_requester(&url, 1, baddies);
        
        if res.is_none(){//ignore invalid url 404
            continue;
        }

        //scrap urls and imgs on a page
        let res_text = res.unwrap();
        let scraped_urls = extract_urls(&res_text);
        let scraped_imgs = extract_images(&res_text);
        let size = res_text.len();

        //printing links in hashmap, should NOT have dups
        println!("Sucess! -> Size:{}", size);

        //download all images found
        println!("*******Images found within this link*******");
        download_img(&scraped_imgs, downloaded, baddies);

        //write page info to a log file
        log_file.write_fmt(format_args!("URL: {} - Size: {}: ", &url, size)).expect("write url failed");
        log_file.write_fmt(format_args!("URLS List: {:?} ,", &scraped_urls)).expect("write url list failed");
        log_file.write_fmt(format_args!("IMG List: {:?} \n", &scraped_imgs)).expect("write images failed");
        
        let new_page = Rc::new(Page::new(size, scraped_urls, scraped_imgs));
        visited.insert(url, new_page.clone());

        //add unvisited urls from scraped_urls to found_urls
        for new in &new_page.links{

            if !visited.contains_key(new){
                found_urls.push_back(new.to_string());
            }
        }
    
    }

}

fn bfs_scraper_with_limit(link: &str, visited: &mut HashMap<String,Rc<Page>>, downloaded: &mut HashMap<String, Image>, baddies: &mut Vec<String>, mut limit:i32, mut log_file:File){
    let mut found_urls: VecDeque<String> = VecDeque::new();
    found_urls.push_back(link.to_string());

    while !found_urls.is_empty() && limit > 0{
        let url = found_urls.pop_front().unwrap();

        println!("Processing URL...{}", url);      //checking which link is being scraped in case it crashes

        let res = http_requester(&url, 1, baddies);
        
        if res.is_none(){//ignore invalid url 404
            continue;
        }

        //scrap urls and imgs on a page
        let res_text = res.unwrap();
        let scraped_urls = extract_urls(&res_text);
        let scraped_imgs = extract_images(&res_text);
        let size = res_text.len();

        //printing links in hashmap, should NOT have dups
        println!("Sucess! -> Size:{}", size);

        //download all images found
        println!("*******Images found within this link*******");
        download_img(&scraped_imgs, downloaded, baddies);

        //write page info to a log file
        log_file.write_fmt(format_args!("URL: {} - Size: {}: ", &url, size)).expect("write url failed");
        log_file.write_fmt(format_args!("URLS List: {:?} ,", &scraped_urls)).expect("write url list failed");
        log_file.write_fmt(format_args!("IMG List: {:?} \n", &scraped_imgs)).expect("write images failed");
        
        let new_page = Rc::new(Page::new(size, scraped_urls, scraped_imgs));
        visited.insert(url, new_page.clone());

        //add unvisited urls from scraped_urls to found_urls
        for new in &new_page.links{
            if !visited.contains_key(new){
                found_urls.push_back(new.to_string());
            }
        }

        limit -=1;
    
    }

}
fn main() {

    //parsing arguments using CLAP
    let arg_matcher = Command::new("Web Crawl Test")
        .about("Web crawler")
        .author("Min Nguyen")
        //.allow_missing_positional(true)
        .arg(Arg::with_name("max")
            .short('m')
            .long("max")
            .takes_value(true)
            .help("Max number of web page to crawl"))
        .arg(Arg::with_name("url")
            //.required(true)
            .short('u')
            .long("url")
            .takes_value(true)
            .help("The url of the root website to crawl from"))
        .get_matches();
    
    //fetching the url from the user: need to start with http:/ or https:/
    let url = arg_matcher.value_of("url").unwrap();
    let http_head = &(url)[..4];
    
    if http_head.ne("http"){
        print!("Not URL!");
        return;
    }
    
    //see how many page to be crawled
    let max = arg_matcher.value_of("max");
    let mut limit = match max {
        None => {
            println!("No limit!");
            0
        },
        Some(s) => {
            match s.parse::<i32>(){
                Ok(n) => {
                    if n <= 0 {
                        println!("No negative nor zero");
                        return;
                    }
                    println!("Crawling {} pages...", n);
                    n
                },
                Err(_) =>{
                    println!("Not an integer");
                    return;
                }
            }
        }
    };


    //list of visited website
    let mut visited: HashMap<String, Rc<Page>> = HashMap::new();
    //list of downloaded images
    let mut downloaded: HashMap<String, Image> = HashMap::new();
    //list of failed URLs
    let mut baddies: Vec<String> = Vec::new();

    //file to write results to
    let mut log_file = File::create("log.txt").unwrap();
    let pages_file = File::create("visited.json").unwrap();
    let imgs_file = File::create("downloaded.json").unwrap();
    let fails_file = File::create("baddies.json").unwrap();
    
    //recursive_scraper(&url, &mut visited, &mut downloaded, &mut baddies);
    if limit == 0{
        bfs_scraper(&url, &mut visited, &mut downloaded, &mut baddies, log_file);
    }else{
        bfs_scraper_with_limit(&url, &mut visited, &mut downloaded, &mut baddies, limit, log_file);
    }
    

    //serialize result as JSON string to the created paths
    let pages_cerealizer = serde_json::ser::to_writer_pretty(pages_file, &visited).unwrap();
    let imgs_cerealizer = serde_json::ser::to_writer_pretty(imgs_file, &downloaded).unwrap();
    let fail_cerealizer = serde_json::ser::to_writer_pretty(fails_file, &baddies).unwrap();


}

/*
serde to serialize data
pull request 
*/