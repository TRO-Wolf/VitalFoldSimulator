## Project prompt
Lets start the build a claude.md file for a simulator project. I want this to be built Rust. I need the simulator to populate data basesd on the sql schema I have provided. I think the best idea is for this to be based on the Actix framework because I'm going to deploy it to Render.com.

The long term goal is a data pipeline that uses AWS, I need to build a really good portfolio project and in order 
to make that happen I need really good data so we're building this simulator to populate that data.
The idea being that data will be coming in from different micro regions of the country.
The health care company will be based out of Florida

Now I will also need to use DynamoDB in some fashion so I have included the 2 tables
I want to use for DynamoDB in the dynamo.json file



## Human written tech stack
- Aurora DSQL for cost optimization and serverless
- Rust + Actix for performance
- render.com for hosting

## general requirements
- ability to turn the engine off or on through an api
- basic api token authentication
- a way to generate fake names for patients
