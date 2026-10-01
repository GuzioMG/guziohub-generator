use std::{char, cmp::max, collections::VecDeque, env, fmt::{Debug, Display, Error}, hint, ops::AddAssign};
use anyhow::{Context, Result, bail, ensure};
use crate::WordSection::SeparatorOrNoPrevious;


pub fn process<'input, 'output>(filecontent: &'input String) -> Result<(Metadata<'output>, VecDeque<StringGaslitAboutItsLength>)>
	where 'input: 'output
{
	let (meta, text) = Metadata::new(filecontent.lines().collect::<Vec<&str>>().as_slice())?;
	let mut walker = Walker::new(text);

	return Ok((meta,
		loop {
			walker = walker.walk().with_context(||"Failure while walking over the content:")?;
			if walker.complete {
				break walker.past_lines;
			}
		}
	));
}


pub trait ApplyToTemplate {
	fn apply_to_template(&self, template: &String) -> String;
}



#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Metadata<'metadata_fields> {
	lang: &'metadata_fields str,
	canonical: &'metadata_fields str,
	title: &'metadata_fields str,
	header: &'metadata_fields str,
	template: &'metadata_fields str,
	defaultclass: &'metadata_fields str,
	description: &'metadata_fields str,
}

impl<'output> Metadata<'output> {
	fn new<'inputs>(lines: &[&'inputs str]) -> Result<(Self, String)>
		where 'inputs: 'output,
	{
		if let [doctype, header, content@.., closing_tag] = lines {
			ensure!(doctype.starts_with("<!DOCTYPE ghtml-v2.0 \"") && doctype.ends_with("\">"), "Invalid G-HTML structure: Invalid doctype! Expected the 1st line to start with „<!DOCTYPE ghtml-v2.0 \"” and end with „\">”, but got „{}” instead.", doctype);
			ensure!(closing_tag.to_string() == "</html>", "Invalid G-HTML structure: No valid closing tag! Expected the last line to be „</html>”, but got „{}” instead.", closing_tag);
			
			let (lang, next_header_segment) = header.strip_prefix("<html flavor=\"ghtml\" lang=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to start with „<html flavor=\"ghtml\" lang=\"”, but got „{}” instead.", header))?
				.split_once("\" canonical=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" canonical=\"” after the the lang param, but got „{}” instead.", header))?;
			let (canonical, next_header_segment) = next_header_segment.split_once("\" title=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" title=\"” after the the canonical param, but got „{}” instead.", header))?;
			let (title, next_header_segment) = next_header_segment.split_once("\" header=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" header=\"” after the the title param, but got „{}” instead.", header))?;
			let (header, next_header_segment) = next_header_segment.split_once("\" template=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" template=\"” after the the header param, but got „{}” instead.", header))?;
			let (template, next_header_segment) = next_header_segment.split_once("\" defaultclass=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" defaultclass=\"” after the the template param, but got „{}” instead.", header))?;
			let (defaultclass, next_header_segment) = next_header_segment.split_once("\" description=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" description=\"” after the the defaultclass param, but got „{}” instead.", header))?;
			let description = next_header_segment.strip_suffix("\">").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to end with a „\">” after the the description param, but got „.....{}” instead.", next_header_segment))?;
	
			return Ok((Metadata{lang, canonical, title, header, template, defaultclass, description}, content.join("\n")));
		} else {
			bail!("Not enough lines provided! Got {}, but expected at least 4.", lines.len());
		}
	}

	pub fn get_template(&self) -> &'output str {
		return self.template;
	}
}

impl ApplyToTemplate for Metadata<'_> {
	fn apply_to_template(&self, template: &String) -> String {
		return template
		.replace("{{PAGE_LANG}}", self.lang)
		.replace("{{CANONICAL_URL}}", self.canonical)
		.replace("{{PAGE_TITLE}}", self.title)
		.replace("{{PAGE_HEADER}}", self.header)
		.replace("{{_INTERNAL_DEFAULT_CLASS}}", self.defaultclass)
		.replace("{{PAGE_DESCRIPTION}}", self.description)
	}
}



#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Walker {
	//Walker state
	index: usize,
	complete: Complete,
	on: String,

	//Input line state
	indent: StringGaslitAboutItsLength,
	indent_completion: Complete,
	active_tags: Vec<ParameterizedHtmlTag>,
	
	//Collected data (Line = output line!)
	past_lines: VecDeque<StringGaslitAboutItsLength>,
	active_line: StringGaslitAboutItsLength,
	word: VecDeque<WordSection>,
	first_word: bool,
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringGaslitAboutItsLength {
	length: usize,
	content: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WordSection {
	#[default] SeparatorOrNoPrevious,
	Literal(String),
	HtmlTag(HtmlTag, Complete),
	VarReplacement(String, Complete),
	HtmlEntity(String, Complete),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum HtmlTag {
	#[default] JustStarted,
	Opening(ParameterizedHtmlTag),
	Closing(String),
	SelfClosing(ParameterizedHtmlTag),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParameterizedHtmlTag {
	tag: String,
	args: Option<String>,
}

trait Renderable {
	fn render(&self) -> Result<StringGaslitAboutItsLength>;
}

type Complete = bool;


impl AddAssign for StringGaslitAboutItsLength{
	fn add_assign(&mut self, rhs: Self) {
		self.length+=rhs.length;
		self.content.push_str(rhs.content.as_str());
	}
}

impl Debug for StringGaslitAboutItsLength {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return std::fmt::Display::fmt(&format!("{} | {}", self.length, self.content), f);
	}
}

impl Display for StringGaslitAboutItsLength {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return std::fmt::Display::fmt(&self.content, f);
	}
}

impl Display for WordSection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self.render() {
			Ok(result) => std::fmt::Display::fmt(&result, f),
			Err(_) => Err(Error)
		}
	}
}

impl Display for HtmlTag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self.render() {
			Ok(result) => std::fmt::Display::fmt(&result, f),
			Err(_) => Err(Error)
		}
	}
}

impl Renderable for HtmlTag {
	fn render(&self) -> Result<StringGaslitAboutItsLength> {
		let content = match self {
			HtmlTag::JustStarted => bail!("Cannot render an incomplete tag!"),
			HtmlTag::Closing(tag) => format!("</{}>", tag),
			HtmlTag::Opening(tag) | HtmlTag::SelfClosing(tag) => {
				let ending = match self {
					HtmlTag::SelfClosing(_) => "/>",
					_ => ">"
				};
				match &tag.args {
					Some(args) => format!("<{} {}{}", tag.tag, args, ending),
					None => format!("<{}{}", tag.tag, ending),
				}
			}
		};

		//TODO: Do envar replacement in tag params

		let length: usize = 0; //TODO: Add a tag param that overrides the size (for eg. emoji-style images, that should be rendered in-line as if they were letters (size 1 instead of 0) or for dummy tags that simply do nothing except for adding -1 to account for stuff like ligatures)

		return Ok(StringGaslitAboutItsLength { length, content });
	}
}

impl Renderable for WordSection {
	fn render(&self) -> Result<StringGaslitAboutItsLength> {
		return match self {
			WordSection::Literal(literal) => Ok(StringGaslitAboutItsLength{length: literal.chars().count(), content: literal.to_string()}),
			WordSection::HtmlTag(tag, true) => tag.render(),
			WordSection::HtmlEntity(entity, true) => Ok(StringGaslitAboutItsLength{length: 1, content: format!("&{};", entity)}),

			WordSection::VarReplacement(varname, true) => match env::var(varname) {
				Ok(content) => Ok(StringGaslitAboutItsLength{length: content.chars().count(), content: content.replace(" ", "&nbsp;")}),
				Err(env::VarError::NotPresent) =>  bail!("Attempted to render an envar %{}% that doesn't exist!", varname),
				Err(env::VarError::NotUnicode(os_string)) => {
					let rs_string = os_string.to_string_lossy();
					return Ok(StringGaslitAboutItsLength{length: rs_string.chars().count(), content: rs_string.to_string().replace(" ", "&nbsp;")});
				}
			}

			WordSection::HtmlTag(_, false) | WordSection::VarReplacement(_, false) | WordSection::HtmlEntity(_, false) => bail!("Attempted to render an incomplete WordSection!"),
			SeparatorOrNoPrevious => WordSection::HtmlEntity("nbsp".to_string(), true).render()
		}
	}
}

impl From<WordSection> for VecDeque<WordSection> {
	fn from(value: WordSection) -> Self {
		let mut vec = VecDeque::new();
		vec.push_back(value);
		return vec;
	}
}

impl StringGaslitAboutItsLength {
	pub fn unwrap(self) -> (String, usize) {
		return (self.content, self.length);
	}

	pub fn find_longest(&self, longest: &mut usize) {
		*longest = max(*longest, self.length);
	}
}

impl WordSection {
	fn new(previous: Self, chr: char) -> Result<VecDeque<Self>> {
		let current = match chr {
			'\n' => bail!("Cannot construct a new word section when switching lines!"),
			' ' => WordSection::SeparatorOrNoPrevious,
			'<' => WordSection::HtmlTag(HtmlTag::JustStarted, false),
			'%' => WordSection::VarReplacement("".to_string(), false),
			'&' => WordSection::HtmlEntity("".to_string(), false),
			_ => WordSection::Literal(chr.to_string())
		};

		return match previous {

			WordSection::Literal(mut base) => match current {
				WordSection::Literal(_) => {
					base.push(chr);
					return Ok(VecDeque::from(WordSection::Literal(base)));
				}
				_ => {
					let mut vec = VecDeque::from(WordSection::Literal(base));
					vec.push_back(current);
					return Ok(vec);
				},
			}

			WordSection::HtmlTag(tag, false) => match tag {
				HtmlTag::JustStarted => match chr {
					'a'|'b'|'c'|'d'|'e'|'f'|'g'|'h'|'i'|'j'|'k'|'l'|'m'|'n'|'o'|'p'|'q'|'r'|'s'|'t'|'u'|'v'|'w'|'x'|'y'|'z' => Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Opening(ParameterizedHtmlTag { tag: chr.to_string(), args: None }), false))),
					'/' => Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Closing("".to_string()), false))),
					_ => bail!("At the beginning of an HTML tag, only a-z alphanumerics (for opening/self-closing tags) and „/” (for closing tags) are allowed, but instead got „{}”.", chr)
				}
				_ => match chr {
					'>' => Ok(VecDeque::from(WordSection::HtmlTag(tag, true))),
					_ => match tag {
						HtmlTag::SelfClosing(_) => bail!("In a self-closing HTML tag, only „>” may come after the „/”, but instead got „{}”.", chr),
						HtmlTag::Opening(ParameterizedHtmlTag { tag, args: Some(mut args) }) => {
							if chr == '/' && (args.chars().filter(|&c| c == '"').count() % 2 == 0) {
								return Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::SelfClosing(ParameterizedHtmlTag { tag, args: Some(args) }), false)));
							}
							args.push(chr);
							return Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Opening(ParameterizedHtmlTag { tag, args: Some(args) }), false)));
						}
						HtmlTag::Opening(ParameterizedHtmlTag { mut tag, args: None }) => match chr {
							'a'|'b'|'c'|'d'|'e'|'f'|'g'|'h'|'i'|'j'|'k'|'l'|'m'|'n'|'o'|'p'|'q'|'r'|'s'|'t'|'u'|'v'|'w'|'x'|'y'|'z'|'-' => {
								tag.push(chr);
								return Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Opening(ParameterizedHtmlTag { tag, args: None }), false)));
							}
							' ' => Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Opening(ParameterizedHtmlTag { tag, args: Some("".to_string()) }), false))),
							'/' => Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::SelfClosing(ParameterizedHtmlTag { tag, args: None }), false))),
							_ => bail!("Before the arguments section in an opening/self-closing HTML tag, only a-z alphanumerics, a „-”, a space (to signal the beginning of said arguments section), a „>” (to signal tag ending), and a „/” (to mark it as a self-closing tag) are allowed, but instead got „{}”.", chr)
						}
						HtmlTag::Closing(mut tag) => match chr {
							'a'|'b'|'c'|'d'|'e'|'f'|'g'|'h'|'i'|'j'|'k'|'l'|'m'|'n'|'o'|'p'|'q'|'r'|'s'|'t'|'u'|'v'|'w'|'x'|'y'|'z'|'-' => {
								tag.push(chr);
								return Ok(VecDeque::from(WordSection::HtmlTag(HtmlTag::Closing(tag), false)));
							}
 							_ => bail!("After the arguments section in a closing HTML tag, only a-z alphanumerics and a „-”, and a „>” (to signal tag ending) are allowed, but instead got „{}”.", chr)
						}
						// SAFETY: This unsafe code is unreachable because it's a part of fallback path, for when we already determined earlier, that HtmlTag: isn't :JustStarted.
						HtmlTag::JustStarted => unsafe{hint::unreachable_unchecked()},
					}
				}
			}

			WordSection::VarReplacement(mut base, false) => match current {
				WordSection::Literal(_) => {
					base.push(chr);
					return Ok(VecDeque::from(WordSection::VarReplacement(base, false)));
				}
				WordSection::VarReplacement(_, _) => Ok(VecDeque::from(WordSection::VarReplacement(base, true))),
				_ => bail!("Cannot use {} inside a var-replacement!", chr),
			}

			WordSection::HtmlEntity(mut ent, false) => match chr {
				'a'|'b'|'c'|'d'|'e'|'f'|'g'|'h'|'i'|'j'|'k'|'l'|'m'|'n'|'o'|'p'|'q'|'r'|'s'|'t'|'u'|'v'|'w'|'x'|'y'|'z'|'0'|'1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9'|'#'|'A'|'B'|'C'|'D'|'E'|'F' => {
					ent.push(chr);
					return Ok(VecDeque::from(WordSection::HtmlEntity(ent, false)));
				}
				';' => Ok(VecDeque::from(WordSection::HtmlEntity(ent, true))),
				_ => bail!("In an HTML entity, only a-z+A-F+0-9 alphanumerics, a „#”, and a „;” are allowed after the „&”, but instead got a „{}”.", chr)
			}

			WordSection::HtmlEntity(_, true) | WordSection::VarReplacement(_, true) | WordSection::HtmlTag(_, true) => {
				let mut vec = VecDeque::from(previous);
				if chr == ' ' {
					vec.push_back(SeparatorOrNoPrevious);
				}
				else {
					vec.push_back(Self::new(SeparatorOrNoPrevious, chr)?.pop_front().with_context(|| "Something went horribly wrong when processing WordSection::from_char - it returned an empty vec, but is should never do so.")?);
				}
				return Ok(vec);
			}

			SeparatorOrNoPrevious => match current {
				SeparatorOrNoPrevious => Ok(VecDeque::from(WordSection::HtmlEntity("nbsp".to_string(), true))),
				_ => Ok(VecDeque::from(current)),
			}
		}
	}
}

impl Walker {
	fn walk(mut self) -> Result<Self> {
		//Known special chars
		let indent_chars = ['|', ' ', '\\', '*', '-', '[', '/'];

		//Setup
		ensure!(!self.complete, "Tried to continue walking even after the walker reached the end!");
		let indexable_on = self.on.chars().collect::<Vec<char>>();
		let current = indexable_on.get(self.index).with_context(|| format!("Tried to index at position {} (+1 because 0-indexed arrays) for a string „{}” that only has {} characters! Note, that this error normally should never happen because an earlier \"Is complete?\" check should've caught it. Something must've gone seriously wrong (Were \"on:\", \"complete\" or \"index\" unsafely messed with? Was a zero-length string passed to work on?).", self.index, self.on, indexable_on.len()))?;

		//Main logic
		if self.active_line.length == 0 && self.word.is_empty(){
			//dbg!(format!("At char „{}” (#{} in „{})”, we're at a beginning of a new line.", current, self.index, self.on));

			//State sanity-check and reset
			ensure!(self.active_tags.is_empty(), "Tried to start a new line (at char {}, #{} in „{}”), but some tags remained unclosed on the previous line!.", current, self.index, self.on);
			self.indent_completion = false;
			self.indent.content = "".to_string();
			self.indent.length = 0;
			self.first_word = true;

			//Line init strategies
			if *current == '\n' {
				//dbg!(format!("It seems to be empty!"));
				self.past_lines.push_back(StringGaslitAboutItsLength { length: 0, content: "".to_string() });
			}
			else if indent_chars.contains(current) {
				//dbg!(format!("New line begins with an indent in form of a {}.", current));
				self.append_indent_char(*current).with_context(|| format!("Indent append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?;
			}
			else {
				self.word.push_back(WordSection::new(SeparatorOrNoPrevious, *current).with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?.pop_front().with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”: Something went horribly wrong when processing WordSection::from_char - it returned an empty vec, but is should never do so.", current, self.index, self.on))?);
				self.indent_completion = true;
			}
		} else {
			//dbg!(format!("At char „{}” (#{} in „{})”, we're continuing a line.", current, self.index, self.on));
			if !self.indent_completion && indent_chars.contains(current) {
				//dbg!(format!("Which means we're continuing an indent, in form of a {}.", current));
				self.append_indent_char(*current).with_context(|| format!("Indent append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?;
			}
			else if *current == '\n' {
				//dbg!(format!("...Nevermind, we're ending it."));
				let ctx = format!("Tried to complete a line after char „{}” (#{} in „{}”), but it failed:", current, self.index, self.on);
				self = self.end_word().with_context(||ctx)?;
				self.past_lines.push_back(self.active_line);
				self.active_line = StringGaslitAboutItsLength { length: 0, ..StringGaslitAboutItsLength::default() } //A new value must be assigned because the previous one was moved out of self's ownership. (and also we need to ensure that length=0 so that the „we're at the beginning of a new line” logic runs on the next pass)
			}
			else {
				self.indent_completion = true;
				let mut vec = WordSection::new(self.word.pop_back().unwrap_or(SeparatorOrNoPrevious), *current).with_context(|| format!("WordSection append error at char „{}” (#{} in „{})”:", current, self.index, self.on))?;
				match vec.pop_front() {
					Some(SeparatorOrNoPrevious) => {
						//dbg!(format!("At char „{}” (#{} in „{})”, we're completing a word.", current, self.index, self.on));
						let ctx = format!("Tried to complete a word after char „{}” (#{} in „{}”), but it failed:", current, self.index, self.on);
						self = self.end_word().with_context(||ctx)?;
					}
					Some(section) => {
						self.word.push_back(section);
					}
					None => bail!("WordSection append error at char „{}” (#{} in „{})”: Something went horribly wrong when processing WordSection::from_char - it returned an empty vec, but is should never do so.", current, self.index, self.on)
				}
				match vec.pop_front() {
					Some(SeparatorOrNoPrevious) => {
						//dbg!(format!("At char „{}” (#{} in „{})”, we're completing a word.", current, self.index, self.on));
						let ctx = format!("Tried to complete a word after char „{}” (#{} in „{}”), but it failed:", current, self.index, self.on);
						self = self.end_word().with_context(||ctx)?;
					},
					Some(section) => {
						self.word.push_back(section);
					}
					None => {}
				};
			}
		}

		//Increment and exit
		self.index+=1;
		if self.index == indexable_on.len() {
			let ctx = format!("Tried to complete the walk after char „{}” (#{} in „{}”), but the necessary end_word subtask failed:", current, self.index-1, self.on);
			self = self.end_word().with_context(||ctx)?;
			ensure!(self.active_tags.is_empty(), "Tried to complete the walk after char „{}” (#{} in „{}”), but some tags remained unclosed on the previous line!", current, self.index-1, self.on);
			self.past_lines.push_back(self.active_line);
			self.active_line = StringGaslitAboutItsLength::default();
			self.complete = true;
		}
		return Ok(self);
	}

	fn append_indent_char(&mut self, chr: char) -> Result<()> {
		ensure!(!self.indent_completion, "Tried to append an indent char „{}” to a line that already completed its indent!", chr);

		let chr_str = chr.to_string();
		let nbsp = "&nbsp;";

		self.active_line.content.push_str(match chr {
			' ' => nbsp,
			_ => chr_str.as_str(),
		});
		self.indent.content.push_str(match chr {
			'|' => "|",
			_ => nbsp,
		});

		self.active_line.length+=1;
		self.indent.length+=1;

		return Ok(());
	}

	fn end_word(mut self) -> Result<Self> {
		const LENGTH_LIMIT: usize = 54;

		//STEP 1: Render the sections.
		let mut rendered = StringGaslitAboutItsLength::default();
		let mut future_active_tags = self.active_tags.clone(); //Cloning needed because we want to be able to roll-back to a backup copy (the og active_tags) when needed; that's the whole point of this var existing (if I didn't need a backup, I'd've worked on active_tags directly and AFAIK the borrow checker wouldn't complain).
		loop {
			let section = match self.word.pop_front() {
				Some(section) => section,
				None => break,
			};
			let rendered_section = section.render().with_context(|| format!("Section rendering failed at char #{} in „{}”:", self.index, self.on))?;
			if let WordSection::HtmlTag(tag, true) = section {
				match tag {
					HtmlTag::Opening(tag) => future_active_tags.push(tag),
					HtmlTag::Closing(tag) => {
						let closes = future_active_tags.pop().with_context(|| format!("Tried to close </{}> at char #{} in „{}”, but there wasn't anything to close!", tag, self.index, self.on))?.tag;
						ensure!(tag == closes, "Tried to close </{}> at char #{} in „{}”, but </{}> was expected instead!", tag, self.index, self.on, closes);
					}
					_ => (),
				}
			}
			rendered += rendered_section;
		}

		//STEP 2A: Combine with whatever's already on the line (easy case)
		if !(rendered.length + self.active_line.length + 1 > LENGTH_LIMIT) {
			if self.first_word { self.first_word = false; }
			else { self.active_line += StringGaslitAboutItsLength{length: 1, content: "&nbsp;".to_string()}; }
			self.active_line += rendered;
			self.active_tags = future_active_tags;
			return Ok(self);
		}

		//STEP 2B: line-wrapping (painful) - STEP 3: Terminate previous line
		let mut tags_to_reopen = Vec::default();
		loop {
			let tag = match self.active_tags.pop() {
				Some(tag) => tag,
				None => break,
			};
			self.active_line += HtmlTag::Closing(tags_to_reopen.push_mut(tag).tag.to_string()).render().with_context(|| format!("HTML tag rendering failed at char #{} in „{}”:", self.index, self.on))?;
		}
		self.past_lines.push_back(self.active_line);
		self.active_tags = future_active_tags;

		//STEP 4: Start new line.
		self.active_line = self.indent.clone(); //a "makes the borrow-checker happy"-noob-clone
		loop {
			let tag = match tags_to_reopen.pop() {
				Some(tag) => tag,
				None => break,
			};
			self.active_line += HtmlTag::Opening(tag).render().with_context(|| format!("HTML tag rendering failed at char #{} in „{}”:", self.index, self.on))?;
		}

		//STEP 5: Combine with whatever's on the just-created line, and make sure that it still fits.
		self.active_line += rendered;
		if self.active_line.length > LENGTH_LIMIT {
			bail!("Even after being moved to a new line, „{}” was just too gosh-damn DummyThiccc™ to fit in the {}-char limit (reached {} chars after combining with the indent and re-opened tags)", self.active_line, LENGTH_LIMIT, self.active_line.length);
		} else {
			return Ok(self);
		}
	}

	fn new(target: String) -> Self {
		return Walker { on: target, ..Self::default() };
	}
}