package dev.telegraphic.jbx.graph;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.github.javaparser.JavaParser;
import com.github.javaparser.ParseResult;
import com.github.javaparser.ParseStart;
import com.github.javaparser.ParserConfiguration;
import com.github.javaparser.Providers;
import com.github.javaparser.Range;
import com.github.javaparser.ast.CompilationUnit;
import com.github.javaparser.ast.Node;
import com.github.javaparser.ast.body.ClassOrInterfaceDeclaration;
import com.github.javaparser.ast.body.EnumDeclaration;
import com.github.javaparser.ast.body.MethodDeclaration;
import com.github.javaparser.ast.body.RecordDeclaration;
import com.github.javaparser.ast.body.VariableDeclarator;
import com.github.javaparser.ast.expr.MethodCallExpr;
import com.github.javaparser.ast.expr.StringLiteralExpr;
import com.github.javaparser.ast.expr.LiteralStringValueExpr;
import com.github.javaparser.serialization.JavaParserJsonSerializer;
import jakarta.json.Json;
import java.io.IOException;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class JbxGraph {
    private static final Pattern TOKEN = Pattern.compile("(\\w+)=\\\"((?:\\\\.|[^\\\"])*)\\\"");
    private static final ObjectMapper JSON = new ObjectMapper();

    private JbxGraph() {}

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("usage: JbxGraph dump <file.java> | patch <file.java> --expect-graph-hash <hash> --op <op>");
            System.exit(2);
        }
        String command = args[0];
        Path source = Path.of(args[1]);
        if ("dump".equals(command)) {
            DumpFormat format = DumpFormat.TEXT;
            for (int i = 2; i < args.length; i++) {
                if ("--json".equals(args[i])) {
                    format = DumpFormat.JBX_JSON;
                } else if ("--javaparser-json".equals(args[i])) {
                    format = DumpFormat.JAVAPARSER_JSON;
                } else {
                    System.err.println("unknown graph dump argument: " + args[i]);
                    System.exit(2);
                }
            }
            System.out.print(dump(source, format));
            return;
        }
        if ("patch".equals(command)) {
            patch(source, args);
            return;
        }
        System.err.println("unknown graph command: " + command);
        System.exit(2);
    }

    private static void patch(Path source, String[] args) throws IOException {
        String expectedHash = null;
        boolean javaparserJsonHash = false;
        List<String> ops = new ArrayList<>();
        for (int i = 2; i < args.length; i++) {
            if ("--expect-graph-hash".equals(args[i]) && i + 1 < args.length) {
                expectedHash = args[++i];
            } else if ("--op".equals(args[i]) && i + 1 < args.length) {
                ops.add(args[++i]);
            } else if ("--javaparser-json".equals(args[i])) {
                javaparserJsonHash = true;
            } else {
                System.err.println("unknown graph patch argument: " + args[i]);
                System.exit(2);
            }
        }
        if (expectedHash == null || expectedHash.isBlank()) {
            System.err.println("graph patch requires --expect-graph-hash");
            System.exit(2);
        }
        if (ops.isEmpty()) {
            System.err.println("graph patch requires at least one --op");
            System.exit(2);
        }

        ParsedSource parsed = parse(source);
        String actualHash;
        if (javaparserJsonHash) {
            actualHash = graphHash(javaparserJson(parsed.compilationUnit()));
        } else {
            String graph = textGraph(source, graphHash(graphBody(source, collectNodes(parsed))), collectNodes(parsed));
            actualHash = graphHash(graph);
        }
        if (!expectedHash.equals(actualHash)) {
            System.err.println("graph hash mismatch: expected " + expectedHash + " but was " + actualHash);
            System.exit(1);
        }

        CompilationUnit cu = parsed.compilationUnit();
        List<GraphNode> nodes = collectNodes(parsed);
        for (String op : ops) {
            applySetLiteralValue(cu, nodes, op);
            nodes = collectNodes(new ParsedSource(cu, parsed.text()));
        }
        Files.writeString(source, cu.toString(), StandardCharsets.UTF_8);
        System.out.println("patched " + source);
    }

    private static void applySetLiteralValue(CompilationUnit cu, List<GraphNode> nodes, String opText) {
        Map<String, String> op = parseOperation(opText);
        if (!"set".equals(op.get("kind"))) {
            throw new IllegalArgumentException("unsupported graph patch operation: " + opText);
        }
        String nodeId = required(op, "node");
        if (nodeId.startsWith("#")) {
            nodeId = nodeId.substring(1);
        }
        String field = required(op, "field");
        if (!"value".equals(field)) {
            throw new IllegalArgumentException("only field=\"value\" is supported for now");
        }
        String expected = required(op, "expect");
        String value = required(op, "value");
        String target = nodeId;
        GraphNode graphNode = nodes.stream()
                .filter(n -> n.id().equals(target))
                .findFirst()
                .orElseThrow(() -> new IllegalArgumentException("graph node not found: #" + target));
        if (!"literal".equals(graphNode.kind())) {
            throw new IllegalArgumentException("graph node #" + target + " is not a literal");
        }
        if (!expected.equals(graphNode.value())) {
            throw new IllegalArgumentException("literal #" + target + " expected value \"" + expected + "\" but was \"" + graphNode.value() + "\"");
        }
        AtomicBoolean changed = new AtomicBoolean();
        cu.walk(StringLiteralExpr.class, literal -> {
            if (changed.get()) {
                return;
            }
            if (literal == graphNode.ast()) {
                literal.setString(value);
                changed.set(true);
            }
        });
        if (!changed.get()) {
            throw new IllegalArgumentException("literal #" + target + " is not a string literal; graph patch currently supports only string literal values");
        }
    }

    private static String dump(Path source, DumpFormat format) throws IOException {
        ParsedSource parsed = parse(source);
        if (format == DumpFormat.JAVAPARSER_JSON) {
            return javaparserJson(parsed.compilationUnit());
        }
        List<GraphNode> nodes = collectNodes(parsed);
        String body = graphBody(source, nodes);
        String hash = graphHash(body);
        if (format == DumpFormat.JBX_JSON) {
            return jsonGraph(source, hash, nodes);
        }
        return textGraph(source, hash, nodes);
    }

    private static String textGraph(Path source, String hash, List<GraphNode> nodes) {
        return "jbx-graph v1\n" + "graph-hash " + hash + "\n" + graphBody(source, nodes);
    }

    private static List<GraphNode> collectNodes(ParsedSource parsed) {
        IdentityHashMap<Node, String> ids = new IdentityHashMap<>();
        LinkedHashMap<String, Integer> counts = new LinkedHashMap<>();
        List<GraphNode> nodes = new ArrayList<>();
        parsed.compilationUnit().walk(Node.TreeTraversal.PREORDER, ast -> {
            String kind = kind(ast);
            if (kind == null) {
                return;
            }
            int next = counts.merge(kind, 1, Integer::sum);
            String id = kind + "-" + next;
            ids.put(ast, id);
            String name = name(ast);
            String value = value(ast);
            String parentId = ast.getParentNode().map(ids::get).orElse(null);
            nodes.add(new GraphNode(id, kind, name, value, parentId, range(ast), snippet(parsed.text(), ast), ast));
        });
        return nodes;
    }

    private static String kind(Node ast) {
        if (ast instanceof ClassOrInterfaceDeclaration n) {
            if ("$COMPACT_CLASS".equals(n.getNameAsString())) return null;
            return "class";
        }
        if (ast instanceof RecordDeclaration) return "record";
        if (ast instanceof EnumDeclaration) return "enum";
        if (ast instanceof MethodDeclaration) return "method";
        if (ast instanceof MethodCallExpr) return "call";
        if (ast instanceof VariableDeclarator) return "variable";
        if (ast instanceof LiteralStringValueExpr) return "literal";
        return null;
    }

    private static String name(Node ast) {
        if (ast instanceof ClassOrInterfaceDeclaration n) return n.getNameAsString();
        if (ast instanceof RecordDeclaration n) return n.getNameAsString();
        if (ast instanceof EnumDeclaration n) return n.getNameAsString();
        if (ast instanceof MethodDeclaration n) return n.getNameAsString();
        if (ast instanceof MethodCallExpr n) return n.getNameAsString();
        if (ast instanceof VariableDeclarator n) return n.getNameAsString();
        return null;
    }

    private static String value(Node ast) {
        if (ast instanceof StringLiteralExpr n) return n.asString();
        if (ast instanceof LiteralStringValueExpr n) return n.getValue();
        return null;
    }

    private static SourceRange range(Node ast) {
        Optional<Range> range = ast.getRange();
        if (range.isEmpty()) {
            return null;
        }
        Range r = range.get();
        return new SourceRange(r.begin.line, r.begin.column, r.end.line, r.end.column);
    }

    private static String snippet(String source, Node ast) {
        Optional<Range> range = ast.getRange();
        if (range.isEmpty()) {
            return null;
        }
        String[] lines = source.split("\\R", -1);
        Range r = range.get();
        if (r.begin.line < 1 || r.end.line > lines.length || r.begin.line > r.end.line) {
            return null;
        }
        StringBuilder out = new StringBuilder();
        for (int lineNo = r.begin.line; lineNo <= r.end.line; lineNo++) {
            String line = lines[lineNo - 1];
            int start = lineNo == r.begin.line ? Math.max(0, r.begin.column - 1) : 0;
            int end = lineNo == r.end.line ? Math.min(line.length(), r.end.column) : line.length();
            if (start > end || start > line.length()) {
                return null;
            }
            if (out.length() > 0) {
                out.append('\n');
            }
            out.append(line, start, end);
        }
        String text = out.toString();
        return text.length() > 240 ? text.substring(0, 237) + "..." : text;
    }

    private static String graphBody(Path source, List<GraphNode> nodes) {
        StringBuilder body = new StringBuilder();
        body.append("path ").append(source).append('\n');
        for (GraphNode node : nodes) {
            body.append("node #").append(node.id()).append(" kind=").append(node.kind());
            if (node.name() != null) {
                body.append(" name=\"").append(esc(node.name())).append("\"");
            }
            if (node.value() != null) {
                body.append(" value=\"").append(esc(node.value())).append("\"");
            }
            if (node.parentId() != null) {
                body.append(" parent=\"#").append(node.parentId()).append("\"");
            }
            if (node.range() != null) {
                body.append(" range=\"").append(node.range()).append("\"");
            }
            if (node.snippet() != null) {
                body.append(" snippet=\"").append(esc(node.snippet())).append("\"");
            }
            body.append('\n');
        }
        return body.toString();
    }

    private static String jsonGraph(Path source, String hash, List<GraphNode> nodes) throws IOException {
        ObjectNode root = JSON.createObjectNode();
        root.put("version", "jbx-graph v1");
        root.put("parser", "javaparser");
        root.put("graphHash", hash);
        root.put("path", source.toString());
        ArrayNode nodeArray = root.putArray("nodes");
        for (GraphNode node : nodes) {
            ObjectNode jsonNode = nodeArray.addObject();
            jsonNode.put("id", "#" + node.id());
            jsonNode.put("kind", node.kind());
            if (node.name() != null) jsonNode.put("name", node.name());
            if (node.value() != null) jsonNode.put("value", node.value());
            if (node.parentId() != null) jsonNode.put("parent", "#" + node.parentId());
            if (node.range() != null) {
                ObjectNode range = jsonNode.putObject("range");
                range.put("beginLine", node.range().beginLine());
                range.put("beginColumn", node.range().beginColumn());
                range.put("endLine", node.range().endLine());
                range.put("endColumn", node.range().endColumn());
            }
            if (node.snippet() != null) jsonNode.put("snippet", node.snippet());
        }
        return JSON.writerWithDefaultPrettyPrinter().writeValueAsString(root) + "\n";
    }

    private static String javaparserJson(CompilationUnit cu) {
        StringWriter out = new StringWriter();
        new JavaParserJsonSerializer().serialize(cu, Json.createGenerator(out));
        return out + "\n";
    }

    private static ParsedSource parse(Path source) throws IOException {
        String text = Files.readString(source, StandardCharsets.UTF_8);
        ParserConfiguration config = new ParserConfiguration().setLanguageLevel(ParserConfiguration.LanguageLevel.BLEEDING_EDGE);
        ParseResult<CompilationUnit> result = new JavaParser(config).parse(ParseStart.COMPILATION_UNIT, Providers.provider(text));
        if (!result.isSuccessful() || result.getResult().isEmpty()) {
            StringBuilder message = new StringBuilder("JavaParser failed to parse ").append(source);
            result.getProblems().forEach(problem -> message.append('\n').append(problem));
            throw new IllegalArgumentException(message.toString());
        }
        return new ParsedSource(result.getResult().get(), text);
    }

    private static String graphHash(String graph) {
        return sha256(graph.replaceFirst("(?s)^jbx-graph v1\\ngraph-hash [0-9a-f]+\\n", ""));
    }

    private static String sha256(String value) {
        try {
            java.security.MessageDigest digest = java.security.MessageDigest.getInstance("SHA-256");
            byte[] bytes = digest.digest(value.getBytes(StandardCharsets.UTF_8));
            StringBuilder hex = new StringBuilder(bytes.length * 2);
            for (byte b : bytes) {
                hex.append(String.format("%02x", b));
            }
            return hex.toString();
        } catch (java.security.NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    private static Map<String, String> parseOperation(String text) {
        Map<String, String> result = new LinkedHashMap<>();
        String trimmed = text.trim();
        int firstSpace = trimmed.indexOf(' ');
        result.put("kind", firstSpace < 0 ? trimmed : trimmed.substring(0, firstSpace));
        Matcher matcher = TOKEN.matcher(trimmed);
        while (matcher.find()) {
            result.put(matcher.group(1), unescape(matcher.group(2)));
        }
        return result;
    }

    private static String required(Map<String, String> op, String name) {
        String value = op.get(name);
        if (value == null) {
            throw new IllegalArgumentException("operation missing " + name);
        }
        return value;
    }

    private static String esc(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t");
    }

    private static String unescape(String value) {
        StringBuilder out = new StringBuilder();
        boolean slash = false;
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (slash) {
                out.append(switch (ch) {
                    case 'n' -> '\n';
                    case 'r' -> '\r';
                    case 't' -> '\t';
                    default -> ch;
                });
                slash = false;
            } else if (ch == '\\') {
                slash = true;
            } else {
                out.append(ch);
            }
        }
        if (slash) {
            out.append('\\');
        }
        return out.toString();
    }

    private record ParsedSource(CompilationUnit compilationUnit, String text) {}

    private enum DumpFormat { TEXT, JBX_JSON, JAVAPARSER_JSON }

    private record SourceRange(int beginLine, int beginColumn, int endLine, int endColumn) {
        @Override
        public String toString() {
            return beginLine + ":" + beginColumn + "-" + endLine + ":" + endColumn;
        }
    }

    private record GraphNode(String id, String kind, String name, String value, String parentId, SourceRange range, String snippet, Node ast) {}
}
