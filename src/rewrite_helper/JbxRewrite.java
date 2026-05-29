package dev.telegraphic.jbx.rewrite;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;
import java.util.stream.Collectors;

import org.openrewrite.ExecutionContext;
import org.openrewrite.InMemoryExecutionContext;
import org.openrewrite.Recipe;
import org.openrewrite.Result;
import org.openrewrite.SourceFile;
import org.openrewrite.Validated;
import org.openrewrite.config.Environment;
import org.openrewrite.config.OptionDescriptor;
import org.openrewrite.config.RecipeDescriptor;
import org.openrewrite.internal.InMemoryLargeSourceSet;
import org.openrewrite.java.JavaParser;

public class JbxRewrite {
    public static void main(String[] args) throws Exception {
        Options options = Options.parse(args);
        Environment baseEnvironment = Environment.builder().scanRuntimeClasspath().scanUserHome().build();
        List<String> recipes = options.recipes.stream().map(JbxRewrite::recipeName).collect(Collectors.toList());

        if (options.discover) {
            discover(baseEnvironment, options);
            return;
        }
        if (recipes.isEmpty()) {
            throw new IllegalArgumentException("at least one --recipe is required");
        }

        Recipe recipe = activateRecipe(baseEnvironment, recipes, options.recipeOptions);
        validateRecipe(recipe, options.failOnInvalidRecipes);

        List<Path> javaFiles = listJavaSources(options.sources);
        if (javaFiles.isEmpty()) {
            System.out.println("No Java sources found.");
            return;
        }

        List<Throwable> executionErrors = new ArrayList<>();
        ExecutionContext ctx = new InMemoryExecutionContext(t -> {
            executionErrors.add(t);
            System.err.println("Recipe execution error: " + t.getMessage());
            t.printStackTrace(System.err);
        });
        Path baseDir = Path.of(".").toAbsolutePath().normalize();
        List<SourceFile> sourceFiles = JavaParser.fromJavaVersion()
                .classpath(List.<Path>of())
                .logCompilationWarningsAndErrors(true)
                .build()
                .parse(javaFiles, baseDir, ctx)
                .collect(Collectors.toList());

        List<Result> results = recipe.run(new InMemoryLargeSourceSet(sourceFiles), ctx).getChangeset().getAllResults();
        if (!executionErrors.isEmpty()) {
            throw new IllegalStateException("recipe execution failed with " + executionErrors.size() + " error(s)", executionErrors.get(0));
        }
        writeReport(options.reportDirectory, results, baseDir);
        printSummary(results, options.reportDirectory);
        if (options.json) {
            printJsonSummary(results, options.reportDirectory);
        }
        if (options.apply) {
            applyResults(results, baseDir);
        }
        if (!results.isEmpty() && options.failOnChanges) {
            System.exit(2);
        }
    }

    private static String recipeName(String recipe) {
        int paren = recipe.indexOf('(');
        return paren >= 0 ? recipe.substring(0, paren) : recipe;
    }

    private static Recipe activateRecipe(Environment baseEnvironment, List<String> recipes, Map<String, String> options) {
        if (options.isEmpty()) {
            return baseEnvironment.activateRecipes(recipes);
        }
        if (recipes.size() != 1) {
            throw new IllegalArgumentException("recipe options currently require exactly one active recipe");
        }
        return instantiateParameterizedRecipe(baseEnvironment, recipes.get(0), optionsForRecipe(recipes.get(0), recipes, options));
    }

    private static Recipe instantiateParameterizedRecipe(Environment env, String recipeName, Map<String, String> options) {
        if (recipeName.equals("org.openrewrite.java.ChangePackage")) {
            String oldPackageName = options.get("oldPackageName");
            String newPackageName = options.get("newPackageName");
            if (oldPackageName == null || newPackageName == null) {
                throw new IllegalArgumentException("recipe " + recipeName + " requires oldPackageName and newPackageName");
            }
            Boolean recursive = options.containsKey("recursive") ? Boolean.valueOf(options.get("recursive")) : null;
            return new org.openrewrite.java.ChangePackage(oldPackageName, newPackageName, recursive);
        }
        throw new IllegalArgumentException("parameterized recipe is not yet supported by jbx helper: " + recipeName);
    }

    private static Map<String, String> optionsForRecipe(String recipe, List<String> recipes, Map<String, String> options) {
        Map<String, String> result = new LinkedHashMap<>();
        for (Map.Entry<String, String> entry : options.entrySet()) {
            String key = entry.getKey();
            int dot = key.indexOf('.');
            if (dot > 0) {
                String prefix = key.substring(0, dot);
                if (prefix.equals(recipe) || prefix.equals(simpleName(recipe))) {
                    result.put(key.substring(dot + 1), entry.getValue());
                }
            } else if (recipes.size() == 1) {
                result.put(key, entry.getValue());
            }
        }
        return result;
    }

    private static String simpleName(String recipe) {
        int dot = recipe.lastIndexOf('.');
        return dot >= 0 ? recipe.substring(dot + 1) : recipe;
    }

    private static void validateRecipe(Recipe recipe, boolean failOnInvalidRecipes) {
        Collection<Validated<Object>> validations = recipe.validateAll();
        List<Validated.Invalid<Object>> failures = validations.stream()
                .map(Validated::failures)
                .flatMap(Collection::stream)
                .collect(Collectors.toList());
        if (!failures.isEmpty()) {
            for (Validated.Invalid<Object> failure : failures) {
                System.err.println("Recipe validation error in " + failure.getProperty() + ": " + failure.getMessage());
            }
            if (failOnInvalidRecipes) {
                throw new IllegalArgumentException("recipe validation failed");
            }
        }
    }

    private static List<Path> listJavaSources(List<Path> roots) throws IOException {
        List<Path> javaFiles = new ArrayList<>();
        List<Path> effectiveRoots = roots.isEmpty() ? List.of(Path.of(".")) : roots;
        for (Path root : effectiveRoots) {
            if (!Files.exists(root)) {
                continue;
            }
            if (Files.isRegularFile(root)) {
                if (root.toString().endsWith(".java")) {
                    javaFiles.add(root.toRealPath());
                }
                continue;
            }
            try (var stream = Files.walk(root)) {
                stream.filter(Files::isRegularFile)
                        .filter(path -> path.toString().endsWith(".java"))
                        .filter(path -> !isIgnored(path))
                        .forEach(path -> {
                            try {
                                javaFiles.add(path.toRealPath());
                            } catch (IOException e) {
                                throw new RuntimeException(e);
                            }
                        });
            }
        }
        return javaFiles;
    }

    private static boolean isIgnored(Path path) {
        for (Path part : path) {
            String name = part.toString();
            if (name.equals(".git") || name.equals("target") || name.equals("build") || name.equals(".jbx")) {
                return true;
            }
        }
        return false;
    }

    private static void writeReport(Path reportDirectory, List<Result> results, Path baseDir) throws IOException {
        Files.createDirectories(reportDirectory);
        Path patchFile = reportDirectory.resolve("rewrite.patch");
        try (BufferedWriter writer = Files.newBufferedWriter(patchFile)) {
            for (Result result : results) {
                writer.write(result.diff());
                writer.write("\n");
            }
        }
    }

    private static void printSummary(List<Result> results, Path reportDirectory) {
        System.out.println("Rewrite results: " + results.size() + " change(s)");
        if (!results.isEmpty()) {
            System.out.println("Patch: " + reportDirectory.resolve("rewrite.patch").normalize());
        }
        for (Result result : results) {
            SourceFile before = result.getBefore();
            SourceFile after = result.getAfter();
            String path = before != null ? before.getSourcePath().toString() : after.getSourcePath().toString();
            System.out.println(" - " + path);
        }
    }

    private static void printJsonSummary(List<Result> results, Path reportDirectory) {
        System.out.println("{\"changes\":" + results.size() + ",\"patch\":\"" + jsonEscape(reportDirectory.resolve("rewrite.patch").normalize().toString()) + "\"}");
    }

    private static String jsonEscape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r");
    }

    private static void applyResults(List<Result> results, Path baseDir) throws IOException {
        for (Result result : results) {
            SourceFile before = result.getBefore();
            SourceFile after = result.getAfter();
            if (before == null && after != null) {
                Path target = baseDir.resolve(after.getSourcePath()).normalize();
                Files.createDirectories(target.getParent());
                Files.writeString(target, after.printAll());
            } else if (before != null && after == null) {
                Files.delete(baseDir.resolve(before.getSourcePath()).normalize());
            } else if (before != null && after != null) {
                Path beforePath = baseDir.resolve(before.getSourcePath()).normalize();
                Path afterPath = baseDir.resolve(after.getSourcePath()).normalize();
                if (!beforePath.equals(afterPath)) {
                    Files.delete(beforePath);
                    Files.createDirectories(afterPath.getParent());
                    Files.writeString(afterPath, after.printAll());
                } else {
                    Files.writeString(beforePath, after.printAll());
                }
            }
        }
    }

    private static void discover(Environment env, Options options) {
        Set<String> selected = options.recipes.stream().map(JbxRewrite::recipeName).collect(Collectors.toCollection(TreeSet::new));
        List<RecipeDescriptor> descriptors = env.listRecipeDescriptors().stream()
                .filter(descriptor -> selected.isEmpty() || selected.contains(descriptor.getName()))
                .filter(descriptor -> matchesSearch(descriptor, options))
                .sorted((left, right) -> left.getName().compareTo(right.getName()))
                .limit(options.limit < 0 ? Long.MAX_VALUE : options.limit)
                .collect(Collectors.toList());
        if (options.json) {
            printRecipesJson(descriptors, options);
            return;
        }
        for (RecipeDescriptor descriptor : descriptors) {
            String alias = aliasFor(descriptor.getName(), options.aliases);
            if (alias != null) {
                System.out.println(alias + "\t" + descriptor.getName());
            } else {
                System.out.println(descriptor.getName());
            }
            if (options.detail) {
                System.out.println("  displayName: " + descriptor.getDisplayName());
                if (descriptor.getDescription() != null && !descriptor.getDescription().isBlank()) {
                    System.out.println("  description: " + descriptor.getDescription());
                }
                for (OptionDescriptor option : descriptor.getOptions()) {
                    System.out.println("  option: " + option.getName() + " " + option.getType() + (option.isRequired() ? " required" : ""));
                }
            }
        }
    }

    private static boolean matchesSearch(RecipeDescriptor descriptor, Options options) {
        if (options.search == null || options.search.isBlank()) {
            return true;
        }
        String needle = options.search.toLowerCase();
        String alias = aliasFor(descriptor.getName(), options.aliases);
        return descriptor.getName().toLowerCase().contains(needle)
                || (alias != null && alias.toLowerCase().contains(needle))
                || (descriptor.getDisplayName() != null && descriptor.getDisplayName().toLowerCase().contains(needle))
                || (descriptor.getDescription() != null && descriptor.getDescription().toLowerCase().contains(needle));
    }

    private static String aliasFor(String recipe, Map<String, String> aliases) {
        for (Map.Entry<String, String> entry : aliases.entrySet()) {
            if (entry.getValue().equals(recipe)) {
                return entry.getKey();
            }
        }
        return null;
    }

    private static void printRecipesJson(List<RecipeDescriptor> descriptors, Options options) {
        System.out.println("[");
        for (int i = 0; i < descriptors.size(); i++) {
            RecipeDescriptor descriptor = descriptors.get(i);
            String alias = aliasFor(descriptor.getName(), options.aliases);
            String comma = i + 1 == descriptors.size() ? "" : ",";
            System.out.print("  {\"name\":\"" + jsonEscape(descriptor.getName()) + "\"");
            if (alias != null) {
                System.out.print(",\"short\":\"" + jsonEscape(alias) + "\"");
            }
            System.out.print(",\"displayName\":\"" + jsonEscape(nullToEmpty(descriptor.getDisplayName())) + "\"");
            if (options.detail) {
                System.out.print(",\"description\":\"" + jsonEscape(nullToEmpty(descriptor.getDescription())) + "\"");
                System.out.print(",\"options\":[");
                for (int j = 0; j < descriptor.getOptions().size(); j++) {
                    OptionDescriptor option = descriptor.getOptions().get(j);
                    String optionComma = j + 1 == descriptor.getOptions().size() ? "" : ",";
                    System.out.print("{\"name\":\"" + jsonEscape(option.getName()) + "\",\"type\":\"" + jsonEscape(option.getType()) + "\",\"required\":" + option.isRequired() + "}" + optionComma);
                }
                System.out.print("]");
            }
            System.out.println("}" + comma);
        }
        System.out.println("]");
    }

    private static String nullToEmpty(String value) {
        return value == null ? "" : value;
    }

    private static final class Options {
        final List<String> recipes = new ArrayList<>();
        final List<Path> sources = new ArrayList<>();
        final Map<String, String> recipeOptions = new TreeMap<>();
        final Map<String, String> aliases = new TreeMap<>();
        Path reportDirectory = Path.of("rewrite");
        String search;
        long limit = -1;
        boolean apply;
        boolean discover;
        boolean detail;
        boolean json;
        boolean failOnChanges;
        boolean failOnInvalidRecipes = true;

        static Options parse(String[] args) {
            Options options = new Options();
            for (int i = 0; i < args.length; i++) {
                switch (args[i]) {
                    case "--recipe" -> options.recipes.add(args[++i]);
                    case "--source" -> options.sources.add(Path.of(args[++i]));
                    case "--option" -> {
                        String option = args[++i];
                        int equals = option.indexOf('=');
                        if (equals <= 0) {
                            throw new IllegalArgumentException("--option must use key=value");
                        }
                        String key = option.substring(0, equals);
                        String value = option.substring(equals + 1);
                        if (options.recipeOptions.containsKey(key)) {
                            System.err.println("warning: duplicate --option key '" + key + "'; overriding previous value");
                        }
                        options.recipeOptions.put(key, value);
                    }
                    case "--report" -> options.reportDirectory = Path.of(args[++i]);
                    case "--apply" -> options.apply = true;
                    case "--dry-run" -> options.apply = false;
                    case "--discover" -> options.discover = true;
                    case "--detail" -> options.detail = true;
                    case "--search" -> options.search = args[++i];
                    case "--limit" -> options.limit = Long.parseLong(args[++i]);
                    case "--alias" -> {
                        String alias = args[++i];
                        int equals = alias.indexOf('=');
                        if (equals <= 0) {
                            throw new IllegalArgumentException("--alias must use short=fully.qualified.Recipe");
                        }
                        options.aliases.put(alias.substring(0, equals), alias.substring(equals + 1));
                    }
                    case "--json" -> options.json = true;
                    case "--fail-on-changes" -> options.failOnChanges = true;
                    case "--no-fail-on-invalid-recipes" -> options.failOnInvalidRecipes = false;
                    default -> throw new IllegalArgumentException("unknown rewrite helper argument: " + args[i]);
                }
            }
            return options;
        }
    }
}
