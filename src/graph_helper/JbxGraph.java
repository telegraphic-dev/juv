package dev.telegraphic.jbx.graph;

import com.github.javaparser.JavaParser;
import com.github.javaparser.ParseResult;
import com.github.javaparser.ParseStart;
import com.github.javaparser.ParserConfiguration;
import com.github.javaparser.Providers;
import com.github.javaparser.ast.CompilationUnit;
import com.github.javaparser.ast.Node;
import com.github.javaparser.serialization.JavaParserJsonDeserializer;
import com.github.javaparser.serialization.JavaParserJsonSerializer;
import jakarta.json.Json;
import jakarta.json.JsonReader;
import java.io.IOException;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

public final class JbxGraph {
    private JbxGraph() {}

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("usage: JbxGraph dump <file.java> | import <file.json> [--output <file.java>]");
            System.exit(2);
        }
        String command = args[0];
        if ("dump".equals(command)) {
            if (args.length != 2) {
                System.err.println("usage: JbxGraph dump <file.java>");
                System.exit(2);
            }
            System.out.print(dump(Path.of(args[1])));
            return;
        }
        if ("import".equals(command)) {
            Path output = null;
            for (int i = 2; i < args.length; i++) {
                if ("--output".equals(args[i]) && i + 1 < args.length) {
                    output = Path.of(args[++i]);
                } else {
                    System.err.println("unknown graph import argument: " + args[i]);
                    System.exit(2);
                }
            }
            String source = importJava(Path.of(args[1]));
            if (output == null) {
                System.out.print(source);
            } else {
                Files.writeString(output, source, StandardCharsets.UTF_8);
            }
            return;
        }
        System.err.println("unknown graph command: " + command);
        System.exit(2);
    }

    private static String dump(Path source) throws IOException {
        StringWriter out = new StringWriter();
        new JavaParserJsonSerializer().serialize(parse(source), Json.createGenerator(out));
        return out + "\n";
    }

    private static String importJava(Path json) throws IOException {
        try (JsonReader reader = Json.createReader(Files.newBufferedReader(json, StandardCharsets.UTF_8))) {
            Node node = new JavaParserJsonDeserializer().deserializeObject(reader);
            if (!(node instanceof CompilationUnit cu)) {
                throw new IllegalArgumentException("JavaParser JSON root is " + node.getClass().getName() + " instead of CompilationUnit");
            }
            return cu.toString();
        }
    }

    private static CompilationUnit parse(Path source) throws IOException {
        String text = Files.readString(source, StandardCharsets.UTF_8);
        ParserConfiguration config = new ParserConfiguration().setLanguageLevel(ParserConfiguration.LanguageLevel.BLEEDING_EDGE);
        ParseResult<CompilationUnit> result = new JavaParser(config).parse(ParseStart.COMPILATION_UNIT, Providers.provider(text));
        if (!result.isSuccessful() || result.getResult().isEmpty()) {
            StringBuilder message = new StringBuilder("JavaParser failed to parse ").append(source);
            result.getProblems().forEach(problem -> message.append('\n').append(problem));
            throw new IllegalArgumentException(message.toString());
        }
        return result.getResult().get();
    }
}
