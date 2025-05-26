fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    haggis.add_object("examples/test/monkey.obj");

    haggis.run();

    Ok(())
}
