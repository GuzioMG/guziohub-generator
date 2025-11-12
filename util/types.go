package util

type TemplateData struct {
	Name    string
	Content *string
}

type LineData struct {
	Length           int
	ProcessedContent string
}

type DocumentData struct {
	Lang        string
	Canonical   string
	Title       string
	Description string
}