package cmd

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"guziohub-generator/util"

	"github.com/spf13/cobra"
)

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   " generate <flags> OR generate <content: File or Folder> <using: File> <to: Folder (unless \"content\" was a File) or Empty Path>",
	Short: "Builds my website",
	Long:  `Builds my website.`,
	Args:  cobra.ExactArgs(3),
	RunE: func(cmd *cobra.Command, args []string) error {
		//Basic setup
		multiContent := false
		toNeedsClosing := true
		if strings.HasPrefix(args[0], "."+string(os.PathSeparator)) || args[0] == "." {
			return errors.New("please use a different form of relative path (like some/subfolder not ./some/subfolder) and/or don't operate directly on the working directory (you CAN do ../[workdir] or use absolute paths for it) to avoid path-parsing issues") // I debugged this and know the cause of it (it's because filepath.Walk tries to be helpful and strips away any references to the workdir from its string path param - but that ends up interfering with my own prefix-stripping logic), but was too lazy to fix
		}

		// Load args
		content, err := os.Open(args[0])
		if err != nil {
			return err
		} else if desc, err := content.Stat(); err != nil {
			return err
		} else if desc.IsDir() {
			multiContent = true
		}
		defer content.Close()

		using, err := os.Open(args[1])
		if err != nil {
			return err
		} else if desc, err := using.Stat(); err != nil {
			return err
		} else if desc.IsDir() {
			return errors.New("the template file (\"using\" arg, aka arg #2) cannot be a directory")
		}
		defer using.Close()

		to, err := os.Open(args[2])
		if err != nil {
			if !os.IsNotExist(err) {
				return err
			}
			toNeedsClosing = false
		} else if desc, err := to.Stat(); err != nil {
			return err
		} else if (desc.IsDir() && !multiContent) || (!desc.IsDir() && multiContent) {
			return errors.New("FS type (File vs Folder) of the output (\"to\" arg, aka arg #3) must match that of the content (\"content\" arg, aka arg #1)")
		}
		defer to.Close() //This may technically return an error if 1) toNeedsClosing is false (ie. the file didn't even exist in the 1st place, so we have nothing to close) or 2) we closed it earlier-than-planned in the (see: "simple case" below). But we don't really care in either case because 1) if it fails to close because of a reason like that - great! it was already closed, so we got the desired end result, anyway 2) it's a defer statement, so it runs after the function already finished, so this won't cause us to return an err != nil and make Cobra display an error to the user, and 3) if we ended up closing it earlier-than-planned, then we have a bigger problem to worry about, anyway (it's about to be re-opened in os.O_EXCL mode, which cannot happen because the file already exists (if it didn't, toNeedsClosing would've gotten set to false, above) - so the user will get an error from that instead, which is more informative than an error from this Close() call would be).

		// Loading template
		template := util.TemplateData{
			Name:    using.Name(),
			Content: nil,
		}
		if templateInfo, err := using.Stat(); err != nil {
			return err
		} else {
			templateBytes := make([]byte, templateInfo.Size())
			if len, err := using.Read(templateBytes); err != nil {
				println("Read \"" + fmt.Sprint(len) + "\" bytes from template \"" + template.Name + "\" (contents: ```html\n" + string(templateBytes) + "\n```), but then got the error below:")
				return err
			} else {
				templateContent := string(templateBytes)
				template.Content = &templateContent
			}
		}

		// Simple case
		if !multiContent {
			if toNeedsClosing {
				to.Close() // Closed earlier-than-planned („planned” by the defer statement above), so we can re-open with O_WRONLY (as opposed to O_RDONLY, which is what os.Open uses)
			}
			if to_wrt, err := util.DeepOpen(args[2], os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0775); err != nil {
				return err
			} else {
				defer to_wrt.Close()
				return process(content, template, to_wrt)
			}
		}

		// Multi-content case
		return filepath.Walk(args[0], func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.IsDir() {
				return nil
			}

			if content_new, err := os.Open(path); err != nil {
				return err
			} else if subpath, success := strings.CutPrefix(path, args[0]); !success {
				content_new.Close()
				return errors.New("could not get subpath from path \"" + path + "\" with base \"" + args[0] + "\"")
			} else if to_new, err := util.DeepOpen(args[2]+string(os.PathSeparator)+subpath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0775); err != nil {
				println("Got subpath \"" + subpath + "\" from path \"" + path + "\" with base \"" + args[0] + "\". This may help debug the error below:")
				content_new.Close()
				return err
			} else {
				defer content_new.Close()
				defer to_new.Close()
				return process(content_new, template, to_new)
			}
		})
	},
}

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// Here you will define your flags and configuration settings.
	// Cobra supports persistent flags, which, if defined here,
	// will be global for your application.

	// rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file (default is $HOME/.guziohub-generator.yaml)")

	// Cobra also supports local flags, which will only run
	// when this action is called directly.
	//rootCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}

func process(content *os.File, template util.TemplateData, to *os.File) error {

	// Loading content
	var contentText *string = nil
	meaningfulLines := 0
	if contentInfo, err := content.Stat(); err != nil {
		return err
	} else {
		contentBytes := make([]byte, contentInfo.Size())
		if len, err := content.Read(contentBytes); err != nil {
			println("Read \"" + fmt.Sprint(len) + "\" bytes from file \"" + content.Name() + "\" (contents: ```html\n" + string(contentBytes) + "\n```), but then got the error below:")
			return err
		} else {
			contentContent := string(contentBytes)
			meaningfulLines = strings.Count(contentContent, "\n") - 2 // Less by 3 over real count to account for the opening, closing and DOCTYPE tags. Subtracting only 2 because Count counts the number of \n, which is always one less than the number of lines.
			if meaningfulLines <= 0 {
				return errors.New("file \"" + content.Name() + "\" does not appear to have any content (got \"" + fmt.Sprint(meaningfulLines) + "\" meaningful lines, ie. after discounting the opening, closing and DOCTYPE tags)")
			}
			contentText = &contentContent
		}
	}

	// Processing content
	parsedLines := make([]util.LineData, meaningfulLines)
	var extractedMetadata *util.DocumentData = nil
	for index, line := range strings.Split(*contentText, "\n") {

		// Skip non-meaningful lines
		if index == 0 || index == 1 || index == meaningfulLines+2 {
			//...Unless they're invalid
			if index == 0 && !(strings.HasPrefix(line, "<!DOCTYPE ghtml-v0.") && strings.Contains(line, " \"") && strings.HasSuffix(line, "\">")) {
				return errors.New("file \"" + content.Name() + "\" does not appear to be a valid G-HTML v0 file (missing, wrong-versioned, or invalid DOCTYPE declaration - found \"" + line + "\" on line #1 instead)")
			} else if index == 1 && !(strings.HasPrefix(line, "<html flavour=\"ghtml\" lang=\"") && strings.Contains(line, "\" canonical=\"") && strings.Contains(line, "\" title=\"") && strings.Contains(line, "\" description=\"") && strings.HasSuffix(line, "\">")) {
				return errors.New("file \"" + content.Name() + "\" does not appear to be a valid HTML file, of G-HTML flavour (missing, mis(s)-attributed, or invalid opening <html> tag - found \"" + line + "\" on line #2 instead)")
			} else if index == meaningfulLines+2 && line != "</html>" {
				return errors.New("file \"" + content.Name() + "\" does not appear to be a valid HTML file, of any (G-HTML, or otherwise) flavour (missing or invalid closing </html> tag - found \"" + line + "\" on line #" + fmt.Sprint(index+1) + " (the last one) instead)")
			}

			//.....Or it happens to be line #2, which - although non-meaningful, as far as content goes - contains metadata we need to extract
			if index == 1 {
				println("Would've extracted metadata from: " + line)
				extractedMetadata = &util.DocumentData{} //TODO: Implement
				strings.Clone(extractedMetadata.Lang)    //no-op to avoid "declared and not used" error
			}

			//................And only THEN skip!
			continue
		}

		// Process meaningful lines
		println("Would've processed: " + line)
		parsedLines[index-2] = util.LineData{
			//TODO: Implement
			Length:           len(line),
			ProcessedContent: line,
		}
	}

	// TODO: Generate CSS from processed lines, combine parsed content lines, inject them+css+metadata into template, and write it to the "to" file

	return nil
}
