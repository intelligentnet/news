# news
A News server with bullet points for topical news items. Used as digital signage feed at http://targetr.net

For you to do :
-	Get an https certificate and create cert.pem and key.pem files
-	Create an OpenAI API account
-	Open an NewsAPI account
-	Set up PostgreSQL with an account username and password
-	Edit the new.sql file and add your account name and run it
-	Get you Postgres access permissions set up for the host running postgres
-	Install latest Rust compiler with rustup
-	Edit the env file and run: . ./env in shell
-	Edit prompt.html file for your URL and desired appearance
-	Create a 'gen' directory for receiving generated files, symlink as 'pic'

Use Targetr where this service is available as an extension.

OR 

Run the server with :

	cargo run --release

The Web site then is available at https://my_URL/ind

Enjoy.

TODO
----

- Document much more
- Add loads of tests
- Add configuration options
- For log running servers Postgres is probably not necessary, add in memory
  persistence
- Add additional News Suppliers
- Refine prompts to LLMs further and make this user accessible
- Review and change instructions for each mode

Contact http://TargetR.net for a full digital signage solution!
