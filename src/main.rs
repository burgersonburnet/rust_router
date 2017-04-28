extern crate iron;
extern crate time;
extern crate router;
extern crate staticfile;
extern crate mount;

use std::path::Path;
use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, typemap, status, Handler};
use iron::error::{IronError};
use time::precise_time_ns;
use staticfile::Static;
use router::{Router, NoRoute};

use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

static GLOBAL_THREAD_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

////////////////MIDDLEWARE////////////////

struct Middleware;

impl typemap::Key for Middleware { type Value = u64; }

impl BeforeMiddleware for Middleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
        req.extensions.insert::<Middleware>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for Middleware {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<Middleware>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }

    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        println!("Hitting custom 404 middleware");

        if let Some(_) = err.error.downcast::<NoRoute>() {
            Ok(Response::with((status::NotFound, "Custom 404 response")))
        } else {
            Err(err)
        }
    }
}

////////////////HANDLERS////////////////

fn hello_world(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((iron::status::Ok, "Hello World")))
}

fn get_index(request: &mut Request) -> IronResult<Response> {
    let counter = GLOBAL_THREAD_COUNT.load(Ordering::SeqCst);
    let react = Static {
        root: Path::new("public/react/").to_path_buf()
    };
    let angular = Static {
        root: Path::new("public/angular/index.html").to_path_buf()
    };

    println!("{:?}", counter);
    let res = if counter % 2 == 0 {
        react.handle(request)
    } else {
        angular.handle(request)
    };
    res
}

// fn handler(req: &mut Request) -> IronResult<Response> {
//     println!("in query");
//     let ref query = req.extensions.get::<Router>().unwrap().find("query").unwrap_or("/");
//     Ok(Response::with((status::Ok, *query)))
// }

////////////////MAIN////////////////

fn main() {
    let mut router = Router::new();

    router.get("/",  get_index, "index");
    router.get("/hello", hello_world, "hello_world");
    router.get("/public/", Static::new(Path::new("public/")), "public");

    let mut router_chain = Chain::new(router);
    router_chain.link_before(Middleware);
    router_chain.link_after(Middleware);

    Iron::new(router_chain).http("localhost:3000").unwrap();
}