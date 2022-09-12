use quick_xml::events::Event;
use quick_xml::reader::Reader;

use tui::{
    text::{Span, Spans},
};

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub struct Article {
    pub title: String,
    pub url: String,
    pub published_at: String,
    pub content: String,
}

#[derive(Debug)]
#[derive(Clone)]
struct Attribute {
    key: String,
    value: String,
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub struct Source {
    pub name: String,
    pub url: String,
}

impl Source {
    pub fn new() -> Self {
        Source {
            name: "".to_string(),
            url: "".to_string(),
        }
    }
}

impl<'a> From<&Source> for Span<'a> {
    fn from(s: &Source) -> Span<'a> {
        Span::raw(s.name.clone())
    }
}

impl<'a> From<&Source> for Spans<'a> {
    fn from(s: &Source) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

impl Article {
    pub fn new() -> Self {
        Article {
            title: "".to_string(),
            url: "".to_string(),
            published_at: "".to_string(),
            content: "".to_string(),
        }
    }
}

impl<'a> From<&Article> for Span<'a> {
    fn from(s: &Article) -> Span<'a> {
        Span::raw(s.title.clone())
    }
}

impl<'a> From<&Article> for Spans<'a> {
    fn from(s: &Article) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

pub fn parse_rss(example: &str) -> Result<Vec<Article>, quick_xml::Error> {
    let mut reader = Reader::from_str(example);
    let mut buf = Vec::new();
    let mut skip_buf = Vec::new();
    let mut articles: Vec<Article> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => match element.name().as_ref() {
                b"item" => {
                    let mut article = Article::new();
                    loop {
                        skip_buf.clear();
                        match reader.read_event_into(&mut skip_buf) {
                            Ok(Event::Start(element)) => match element.name().as_ref() {
                                b"title" => { article.title = reader.read_text(element.name())?.trim().to_string() }
                                b"link" => { article.url = reader.read_text(element.name())?.trim().to_string() }
                                b"pubDate" => { article.published_at = reader.read_text(element.name())?.trim().to_string() }
                                b"description" => { article.content = reader.read_text(element.name())?.trim().to_string() }
                                _ => {}
                            },
                            Ok(Event::End(element)) => {
                                if element.name().as_ref() == b"item" {
                                    articles.push(article);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(articles)
}

pub fn parse_atom(example: &str) -> Result<Vec<Article>, quick_xml::Error> {
    let mut reader = Reader::from_str(example);
    let mut buf = Vec::new();
    let mut skip_buf = Vec::new();
    let mut articles: Vec<Article> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => match element.name().as_ref() {
                b"entry" => {
                    let mut article = Article::new();
                    loop {
                        skip_buf.clear();
                        match reader.read_event_into(&mut skip_buf) {
                            Ok(Event::Empty(ref element)) => match element.name().as_ref() {
                                b"link" => {
                                    let attributes = element
                                        .attributes()
                                        .map(|a| {
                                            let a = a.unwrap();
                                            return Attribute {
                                                key: std::str::from_utf8(a.key.local_name().as_ref()).unwrap().to_string(),
                                                value: std::str::from_utf8(a.value.as_ref()).unwrap().to_string(),
                                            };
                                        })
                                        .collect::<Vec<_>>();

                                    for a in attributes.iter() {
                                        if a.key == "href" {
                                            article.url = a.value.clone();
                                        }
                                    }
                                }
                                _ => {}
                            },
                            Ok(Event::Start(ref element)) => match element.name().as_ref() {
                                b"title" => { article.title = reader.read_text(element.name())?.trim().to_string() }
                                // b"link" => { article.url = reader.read_text(element.name())?.trim().to_string() }
                                // b"link" => {
                                //     // TODO: use the same logic as in the self-closing case...
                                // }
                                b"updated" => { article.published_at = reader.read_text(element.name())?.trim().to_string() }
                                b"content" => { article.content = reader.read_text(element.name())?.trim().to_string() }
                                _ => {}
                            },
                            Ok(Event::End(element)) => {
                                if element.name().as_ref() == b"entry" {
                                    articles.push(article);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;
    mod examples;

    #[test]
    fn can_handle_rss_first_example() -> Result<(), quick_xml::Error> {
        let expected: Vec<Article> = vec![
            Article { title: "Report Validates Impact of Visual AI on Test Automation".to_string(), url: "https://www.infoq.com/news/2020/04/visual-ai-test-automation/?utm_campaign=infoq_content&utm_source=infoq&utm_medium=feed&utm_term=news".to_string(), published_at: "Tue, 21 Apr 2020 16:00:00 GMT".to_string(), content: "<img src=\"https://www.infoq.com/styles/i/logo_bigger.jpg\"/><p>Empirical data from 288 quality engineers across 101 countries provide insight and credibility behind a report demonstrating the benefits of Visual AI in the field of test automation. The report comes from Applitools, a company that sells functional and visual testing tools using visual AI.</p> <i>By Matthew Coughlan</i>".to_string() },
            Article { title: "Microsoft and Google Release New Benchmarks for Cross-Language AI Tasks".to_string(), url: "https://www.infoq.com/news/2020/04/microsoft-google-nlp-benchmarks/?utm_campaign=infoq_content&utm_source=infoq&utm_medium=feed&utm_term=news".to_string(), published_at: "Tue, 21 Apr 2020 13:00:00 GMT".to_string(), content: "<img src=\"https://res.infoq.com/news/2020/04/microsoft-google-nlp-benchmarks/en/headerimage/microsoft-google-nlp-benchmarks-1587309110687.jpg\"/><p>Research teams at Microsoft Research and Google AI have announced new benchmarks for cross-language natural-language understanding (NLU) tasks for AI systems, including named-entity recognition and question answering. Google's XTREME covers 40 languages and includes nine tasks, while Microsoft's XGLUE covers 27 languages and eleven tasks.</p> <i>By Anthony Alford</i>".to_string() },
        ];
        let articles = parse_rss(examples::RSS_EXAMPLE_1)?;
        assert_eq!(expected.len(), articles.len());

        let it = expected.iter().zip(articles.iter());

        for (_i, (x, y)) in it.enumerate() {
            // println!("{}: ({:?}, {:?})", i, x, y);
            assert_eq!(*x, *y);
        }
        Ok(())
    }

    #[test]
    fn can_handle_rss_second_example() -> Result<(), quick_xml::Error> {
        let expected: Vec<Article> = vec![
            Article { title: "Remote 103: Software".to_string(), url: "https://stuartsierra.com/2020/03/09/remote-103-software-real-time-collaboration".to_string(), published_at: "Mon, 09 Mar 2020 13:22:32 +0000".to_string(), content: "<![CDATA[In the first two posts in this series, I talked about hardware: networking and headsets. I’ll come back to hardware eventually, but the next thing on the checklist is software. Again, I’m not going to recommend specific products here. What I will do is provide you with a set of criteria by which to evaluate&#8230; <p><a class=\"moretag\" href=\"https://stuartsierra.com/2020/03/09/remote-103-software-real-time-collaboration\">Read the full article</a></p>]]>".to_string() },
            Article { title: "Remote 102: Headsets".to_string(), url: "https://stuartsierra.com/2020/03/06/remote-102-headsets".to_string(), published_at: "Fri, 06 Mar 2020 18:53:31 +0000".to_string(), content: "<![CDATA[If you saw my last post, you’ve got your computer wired up. Time to get yourself wired up too. The next piece of hardware you need to be a successful remote worker is a headset with an adjustable microphone boom. Everyone has different preferences —&#160;weight, fit, padding, shape — so I’m not going to recommend&#8230; <p><a class=\"moretag\" href=\"https://stuartsierra.com/2020/03/06/remote-102-headsets\">Read the full article</a></p>]]>".to_string() },
        ];
        let articles = parse_rss(examples::RSS_EXAMPLE_2)?;
        assert_eq!(expected.len(), articles.len());

        let it = expected.iter().zip(articles.iter());

        for (_i, (x, y)) in it.enumerate() {
            // println!("{}: ({:?}, {:?})", i, x, y);
            assert_eq!(*x, *y);
        }
        Ok(())
    }

    #[test]
    fn it_can_handle_atom() -> Result<(), quick_xml::Error> {
        let expected: Vec<Article> = vec![
            Article { title: "<![CDATA[ REPL Driven Design ]]>".to_string(), url: "http://blog.cleancoder.com/uncle-bob/2020/05/27/ReplDrivenDesign.html".to_string(), published_at: "2020-05-27T00:00:00+00:00".to_string(), content: "<![CDATA[ <p>Some content for entry 1</p> ]]>".to_string() },
            Article { title: "<![CDATA[ A Little More Clojure ]]>".to_string(), url: "http://blog.cleancoder.com/uncle-bob/2020/04/09/ALittleMoreClojure.html".to_string(), published_at: "2020-04-09T00:00:00+00:00".to_string(), content: "<![CDATA[ <p>Some content for entry 2</p> ]]>".to_string() },
        ];
        let articles = parse_atom(examples::ATOM_EXAMPLE)?;
        assert_eq!(expected.len(), articles.len());

        let it = expected.iter().zip(articles.iter());

        for (_i, (x, y)) in it.enumerate() {
            // println!("{}: ({:?}, {:?})", i, x, y);
            assert_eq!(*x, *y);
        }
        Ok(())
    }
}
