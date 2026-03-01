#!/bin/python3
# read in the quotes file
with open("quotes.md", "r") as quotes_file:
    all_quotes = quotes_file.read()


def sql_escape(s: str) -> str:
    return s.replace("'", "''")


# [body, source_media, writer]
quotes_list = list(
    map(
        lambda quote_line: quote_line.split("|"),
        filter(
            lambda line: len(line) != 0,
            all_quotes.split("\n")
        )
    )
)

insert_statements = [
    f"INSERT INTO quotes (body, source_media, writer) VALUES ('{sql_escape(body)}', '{
        sql_escape(source)}', '{sql_escape(writer)}');"
    for body, source, writer in quotes_list
]

joined_insert_statements = "\n".join(insert_statements)

with open("quotes_schema.sql", "w") as quotes_schema:
    quotes_schema.write("""
DROP TABLE IF EXISTS quotes;
CREATE TABLE IF NOT EXISTS quotes(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    writer TEXT,
    source_media TEXT
);

""")

    quotes_schema.write(joined_insert_statements)
