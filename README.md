# news
A News server with bullet points for topical news items. Used as digital signage feed at http://targetr.net

For you to do :
-	Get an https certificate and create cert.pem and key.pem files
-	Create an OpenAI API account
-	Open an NewsAPI account
-	Set up PostgreSQL with an account username and password
-	Get you Postgres access permissions set up for the host running postgres
-	Install latest Rust compiler with rustup
-	Edit the env file and run: . ./env in shell
-	Edit prompt.html file for your URL and desired appearance
-	Create a 'gen' directory for receiving generated files

Run the server with :

	cargo run --release

The Web site is available at https://my_URL/news

Enjoy.

TODO
----

- Document much more
- Add loads of tests
- Add configuration options
- For log running servers Postgres is probably not necessary, add in memory
  persistence
- Add additional News Suppliers
- Add Additional LLMs to use. Currently GPT-3.5 or can use GPT-4 (expensive)

Contact http://TargetR.net for a full digital signage solution!
