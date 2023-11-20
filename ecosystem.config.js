
module.exports = {
    apps: [{
        name: "binary",
        script: "./target/release/fuber-eats-backend",
        exec_interpreter: "none",
        exec_mode: "fork_mode",
    }]
};

