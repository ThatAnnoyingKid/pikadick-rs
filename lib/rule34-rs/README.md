# rule34-rs
A library to interact with https://rule34.xxx. 
The api for rule34 appears to be based of an old version of gelbooru's.
It contains many bugs and hidden, obsoleted api options that only work on this website.
This library binds over these hidden options in an attempt to make it more useful, at the cost of being not generalizable to other Boorus.

## Api Quirks/Bugs

### JSON API Post Search
When searching for posts, the JSON API will return "" instead of [].
This will make JSON parsing fail, so this case must be special cased if the JSON API is used.

### Weird Tag Searching By Name
When searching for tags by name, some tags with non-alphanumeric characters will not show up even though they exist.
This can be worked around by translating these characters into their xml-escape form.
However, only specific XML escapes work and this cannot be done with all tags.
These escapes need to be discovered experimentally.

## Features
`cli`: Off by default. Used for building the CLI.

## References
 * https://api.rule34.xxx/index.php
 * https://rule34.xxx/index.php?page=forum&s=view&id=13742
 * https://rule34.xxx/index.php?page=help&topic=dapi
 * https://gelbooru.com/index.php?page=wiki&s=view&id=18780