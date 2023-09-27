/*
[dependencies]
serde = { version = "1", features = ["derive"] }
serde-xml-rs = "*"
*/
#![allow(dead_code)]

use serde::Deserialize;
use serde_xml_rs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GC {
    version: String,
    endianness: String,
    db_path: String,
    #[serde(rename = "$value")]
    gc: Vec<Sys>,
}

#[derive(Debug, Deserialize)]
struct Sys {
    #[serde(rename = "$value")]
    sys: Vec<Area>,
}

#[derive(Debug, Deserialize)]
struct Area {
    name: Vec<Name>,
    module: Vec<Module>,
}

#[derive(Debug, Deserialize)]
struct Name {
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Debug, Deserialize)]
struct Module {
    #[serde(rename = "$value")]
    value: String,
}

fn main() {
    let xml = r#"
    <GC Version="x.x.x" Endianness="little" DbPath="xxx/yyy/zzz">
    	<Sys>
    		<Area>
    			<name>AAA</name>
                <name>CCC</name>
    			<module>BBB</module>
    		</Area>
    	</Sys>
    	<Sys>
    		<Area>
    			<name>AAA</name>
    	        <module>BBB</module>
    		</Area>
    	</Sys>
    </GC>
    "#;

    let foo: GC = serde_xml_rs::from_str(xml).unwrap();

    println!("{foo:#?}");
}
