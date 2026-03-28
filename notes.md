# Notes and ToDo for Jefscad

Brief notes about project structure:

The project is a rust workspace with two crates:
* jefscad for the full solid modeling language
* flint for the rounded floating point interval implementation

## Jefscad project

The jefscad project will be a rust crate that uses pyo3 (and maturin?) to create a 
python package that will be the interface for solid modeling.

## Flint project

The flint project defines new rounded floating point interval numeric types.

## ToDo:

[_] Create the cargo workspace
   - [x] Create the flint project
   - [x] add flint project to cargo workspace
   - [x] Create the jefscad project
   - [x] add jefscad project to cargo workspace
   - [x] add copyright and license file
   - [x] initialize git repo, upload to github

