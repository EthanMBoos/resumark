#import "/theme.typ": render

#let input = json("/resume.json")
#render(input.at("document"), input.at("settings"), input.at("theme"))
